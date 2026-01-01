use std::path::PathBuf;

use axum::{
    Extension, Json,
    extract::{Query, State},
    response::Json as ResponseJson,
};
use db::models::{
    execution_process::{ExecutionProcess, ExecutionProcessRunReason},
    merge::{Merge, MergeStatus},
    repo::{Repo, RepoError},
    session::{CreateSession, Session},
    task::{Task, TaskStatus},
    workspace::{Workspace, WorkspaceError},
    workspace_repo::WorkspaceRepo,
};
use deployment::Deployment;
use executors::actions::{
    ExecutorAction, ExecutorActionType, coding_agent_follow_up::CodingAgentFollowUpRequest,
    coding_agent_initial::CodingAgentInitialRequest,
};
use git2::BranchType;
use serde::{Deserialize, Serialize};
use services::services::{
    container::ContainerService,
    git::{GitCliError, GitServiceError},
    git_provider::{self, CreateMrRequest, ProviderError, UnifiedComment},
};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateGitHubPrRequest {
    pub title: String,
    pub body: Option<String>,
    pub target_branch: Option<String>,
    pub draft: Option<bool>,
    pub repo_id: Uuid,
    #[serde(default)]
    pub auto_generate_description: bool,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type", rename_all = "snake_case")]
pub enum CreatePrError {
    GithubCliNotInstalled,
    GithubCliNotLoggedIn,
    GitCliNotLoggedIn,
    GitCliNotInstalled,
    TargetBranchNotFound { branch: String },
}

#[derive(Debug, Serialize, TS)]
pub struct AttachPrResponse {
    pub pr_attached: bool,
    pub pr_url: Option<String>,
    pub pr_number: Option<i64>,
    pub pr_status: Option<MergeStatus>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct AttachExistingPrRequest {
    pub repo_id: Uuid,
}

#[derive(Debug, Serialize, TS)]
pub struct PrCommentsResponse {
    pub comments: Vec<UnifiedComment>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type", rename_all = "snake_case")]
pub enum GetPrCommentsError {
    NoPrAttached,
    GithubCliNotInstalled,
    GithubCliNotLoggedIn,
}

#[derive(Debug, Deserialize, TS)]
pub struct GetPrCommentsQuery {
    pub repo_id: Uuid,
}

pub const DEFAULT_PR_DESCRIPTION_PROMPT: &str = r#"Update the GitHub PR that was just created with a better title and description.
The PR number is #{pr_number} and the URL is {pr_url}.

Analyze the changes in this branch and write:
1. A concise, descriptive title that summarizes the changes, postfixed with "(Vibe Kanban)"
2. A detailed description that explains:
   - What changes were made
   - Why they were made (based on the task context)
   - Any important implementation details
   - At the end, include a note: "This PR was written using [Vibe Kanban](https://vibekanban.com)"

Use `gh pr edit` to update the PR."#;

async fn trigger_pr_description_follow_up(
    deployment: &DeploymentImpl,
    workspace: &Workspace,
    pr_number: i64,
    pr_url: &str,
) -> Result<(), ApiError> {
    // Get the custom prompt from config, or use default
    let config = deployment.config().read().await;
    let prompt_template = config
        .pr_auto_description_prompt
        .as_deref()
        .unwrap_or(DEFAULT_PR_DESCRIPTION_PROMPT);

    // Replace placeholders in prompt
    let prompt = prompt_template
        .replace("{pr_number}", &pr_number.to_string())
        .replace("{pr_url}", pr_url);

    drop(config); // Release the lock before async operations

    // Get or create a session for this follow-up
    let session =
        match Session::find_latest_by_workspace_id(&deployment.db().pool, workspace.id).await? {
            Some(s) => s,
            None => {
                Session::create(
                    &deployment.db().pool,
                    &CreateSession { executor: None },
                    Uuid::new_v4(),
                    workspace.id,
                )
                .await?
            }
        };

    // Get executor profile from the latest coding agent process in this session
    let executor_profile_id =
        ExecutionProcess::latest_executor_profile_for_session(&deployment.db().pool, session.id)
            .await?;

    // Get latest agent session ID if one exists (for coding agent continuity)
    let latest_agent_session_id = ExecutionProcess::find_latest_coding_agent_turn_session_id(
        &deployment.db().pool,
        session.id,
    )
    .await?;

    let working_dir = workspace
        .agent_working_dir
        .as_ref()
        .filter(|dir| !dir.is_empty())
        .cloned();

    // Build the action type (follow-up if session exists, otherwise initial)
    let action_type = if let Some(agent_session_id) = latest_agent_session_id {
        ExecutorActionType::CodingAgentFollowUpRequest(CodingAgentFollowUpRequest {
            prompt,
            session_id: agent_session_id,
            executor_profile_id: executor_profile_id.clone(),
            working_dir: working_dir.clone(),
        })
    } else {
        ExecutorActionType::CodingAgentInitialRequest(CodingAgentInitialRequest {
            prompt,
            executor_profile_id: executor_profile_id.clone(),
            working_dir,
        })
    };

    let action = ExecutorAction::new(action_type, None);

    deployment
        .container()
        .start_execution(
            workspace,
            &session,
            &action,
            &ExecutionProcessRunReason::CodingAgent,
        )
        .await?;

    Ok(())
}

/// Create merge request (PR for GitHub, MR for GitLab)
/// Provider is auto-detected from repository remote URL
pub async fn create_github_pr(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<CreateGitHubPrRequest>,
) -> Result<ResponseJson<ApiResponse<String, CreatePrError>>, ApiError> {
    let pool = &deployment.db().pool;

    let workspace_repo =
        WorkspaceRepo::find_by_workspace_and_repo_id(pool, workspace.id, request.repo_id)
            .await?
            .ok_or(RepoError::NotFound)?;

    let repo = Repo::find_by_id(pool, workspace_repo.repo_id)
        .await?
        .ok_or(RepoError::NotFound)?;

    let repo_path = repo.path;
    let target_branch = if let Some(branch) = request.target_branch {
        branch
    } else {
        workspace_repo.target_branch.clone()
    };

    let container_ref = deployment
        .container()
        .ensure_container_exists(&workspace)
        .await?;
    let workspace_path = PathBuf::from(&container_ref);
    let worktree_path = workspace_path.join(repo.name);

    match deployment
        .git()
        .check_remote_branch_exists(&repo_path, &target_branch)
    {
        Ok(false) => {
            return Ok(ResponseJson(ApiResponse::error_with_data(
                CreatePrError::TargetBranchNotFound {
                    branch: target_branch.clone(),
                },
            )));
        }
        Err(GitServiceError::GitCLI(GitCliError::AuthFailed(_))) => {
            return Ok(ResponseJson(ApiResponse::error_with_data(
                CreatePrError::GitCliNotLoggedIn,
            )));
        }
        Err(GitServiceError::GitCLI(GitCliError::NotAvailable)) => {
            return Ok(ResponseJson(ApiResponse::error_with_data(
                CreatePrError::GitCliNotInstalled,
            )));
        }
        Err(e) => return Err(ApiError::GitService(e)),
        Ok(true) => {}
    }

    // Push the branch to remote first (GitHub/GitLab agnostic)
    if let Err(e) = deployment
        .git()
        .push_to_github(&worktree_path, &workspace.branch, false)
    {
        tracing::error!("Failed to push branch to remote: {}", e);
        match e {
            GitServiceError::GitCLI(GitCliError::AuthFailed(_)) => {
                return Ok(ResponseJson(ApiResponse::error_with_data(
                    CreatePrError::GitCliNotLoggedIn,
                )));
            }
            GitServiceError::GitCLI(GitCliError::NotAvailable) => {
                return Ok(ResponseJson(ApiResponse::error_with_data(
                    CreatePrError::GitCliNotInstalled,
                )));
            }
            _ => return Err(ApiError::GitService(e)),
        }
    }

    let norm_target_branch_name = if matches!(
        deployment
            .git()
            .find_branch_type(&repo_path, &target_branch)?,
        BranchType::Remote
    ) {
        // Remote branches are formatted as {remote}/{branch} locally.
        // For MR/PR APIs, we must provide just the branch name.
        let remote = deployment
            .git()
            .get_remote_name_from_branch_name(&worktree_path, &target_branch)?;
        let remote_prefix = format!("{}/", remote);
        target_branch
            .strip_prefix(&remote_prefix)
            .unwrap_or(&target_branch)
            .to_string()
    } else {
        target_branch
    };
    // Create the MR/PR using provider abstraction
    let pr_request = CreateMrRequest {
        title: request.title.clone(),
        body: request.body.clone(),
        head_branch: workspace.branch.clone(),
        base_branch: norm_target_branch_name.clone(),
        draft: request.draft,
    };

    // Detect provider and create appropriate service
    let provider = git_provider::create_provider(&repo_path)
        .map_err(|e| ApiError::GitService(GitServiceError::InvalidRepository(e.to_string())))?;
    let (_, repo_id) = git_provider::detect_provider(&repo_path)
        .map_err(|e| ApiError::GitService(GitServiceError::InvalidRepository(e.to_string())))?;

    match provider.create_merge_request(&repo_id, &pr_request).await {
        Ok(pr_info) => {
            // Update the workspace with PR information
            if let Err(e) = Merge::create_pr(
                pool,
                workspace.id,
                workspace_repo.repo_id,
                &norm_target_branch_name,
                pr_info.number as i64,
                &pr_info.url,
            )
            .await
            {
                tracing::error!("Failed to update workspace PR status: {}", e);
            }

            // Auto-open PR/MR in browser
            if let Err(e) = utils::browser::open_browser(&pr_info.url).await {
                tracing::warn!("Failed to open MR/PR in browser: {}", e);
            }
            deployment
                .track_if_analytics_allowed(
                    "github_pr_created",
                    serde_json::json!({
                        "workspace_id": workspace.id.to_string(),
                    }),
                )
                .await;

            // Trigger auto-description follow-up if enabled
            if request.auto_generate_description
                && let Err(e) = trigger_pr_description_follow_up(
                    &deployment,
                    &workspace,
                    pr_info.number as i64,
                    &pr_info.url,
                )
                .await
            {
                tracing::warn!(
                    "Failed to trigger PR description follow-up for attempt {}: {}",
                    workspace.id,
                    e
                );
            }

            Ok(ResponseJson(ApiResponse::success(pr_info.url)))
        }
        Err(e) => {
            tracing::error!(
                "Failed to create MR/PR for attempt {}: {}",
                workspace.id,
                e
            );
            match &e {
                ProviderError::NotInstalled { .. } => Ok(ResponseJson(
                    ApiResponse::error_with_data(CreatePrError::GithubCliNotInstalled),
                )),
                ProviderError::NotAuthenticated(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(CreatePrError::GithubCliNotLoggedIn),
                )),
                _ => Err(ApiError::GitService(GitServiceError::InvalidRepository(e.to_string()))),
            }
        }
    }
}

pub async fn attach_existing_pr(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
    Json(request): Json<AttachExistingPrRequest>,
) -> Result<ResponseJson<ApiResponse<AttachPrResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let task = workspace
        .parent_task(pool)
        .await?
        .ok_or(ApiError::Workspace(WorkspaceError::TaskNotFound))?;

    let workspace_repo =
        WorkspaceRepo::find_by_workspace_and_repo_id(pool, workspace.id, request.repo_id)
            .await?
            .ok_or(RepoError::NotFound)?;

    let repo = Repo::find_by_id(pool, workspace_repo.repo_id)
        .await?
        .ok_or(RepoError::NotFound)?;

    // Check if PR already attached for this repo
    let merges = Merge::find_by_workspace_and_repo_id(pool, workspace.id, request.repo_id).await?;
    if let Some(Merge::Pr(pr_merge)) = merges.into_iter().next() {
        return Ok(ResponseJson(ApiResponse::success(AttachPrResponse {
            pr_attached: true,
            pr_url: Some(pr_merge.pr_info.url.clone()),
            pr_number: Some(pr_merge.pr_info.number),
            pr_status: Some(pr_merge.pr_info.status.clone()),
        })));
    }

    // Detect provider and create appropriate service
    let provider = git_provider::create_provider(&repo.path)
        .map_err(|e| ApiError::GitService(GitServiceError::InvalidRepository(e.to_string())))?;
    let (_, repo_id) = git_provider::detect_provider(&repo.path)
        .map_err(|e| ApiError::GitService(GitServiceError::InvalidRepository(e.to_string())))?;

    // List all MRs/PRs for branch (open, closed, and merged)
    let prs = provider
        .list_mrs_for_branch(&repo_id, &workspace.branch)
        .await
        .map_err(|e| ApiError::GitService(GitServiceError::InvalidRepository(e.to_string())))?;

    // Take the first MR/PR (prefer open, but also accept merged/closed)
    if let Some(pr_info) = prs.into_iter().next() {
        // Save PR info to database
        let merge = Merge::create_pr(
            pool,
            workspace.id,
            workspace_repo.repo_id,
            &workspace_repo.target_branch,
            pr_info.number as i64,
            &pr_info.url,
        )
        .await?;

        // Convert PrState to MergeStatus
        let merge_status: MergeStatus = pr_info.state.into();

        // Update status if not open
        if !matches!(merge_status, MergeStatus::Open) {
            Merge::update_status(
                pool,
                merge.id,
                merge_status.clone(),
                pr_info.merge_commit_sha.clone(),
            )
            .await?;
        }

        // If MR/PR is merged, mark task as done
        if matches!(merge_status, MergeStatus::Merged) {
            Task::update_status(pool, task.id, TaskStatus::Done).await?;

            // Try broadcast update to other users in organization
            if let Ok(publisher) = deployment.share_publisher() {
                if let Err(err) = publisher.update_shared_task_by_id(task.id).await {
                    tracing::warn!(
                        ?err,
                        "Failed to propagate shared task update for {}",
                        task.id
                    );
                }
            } else {
                tracing::debug!(
                    "Share publisher unavailable; skipping remote update for {}",
                    task.id
                );
            }
        }

        Ok(ResponseJson(ApiResponse::success(AttachPrResponse {
            pr_attached: true,
            pr_url: Some(pr_info.url),
            pr_number: Some(pr_info.number as i64),
            pr_status: Some(merge_status),
        })))
    } else {
        Ok(ResponseJson(ApiResponse::success(AttachPrResponse {
            pr_attached: false,
            pr_url: None,
            pr_number: None,
            pr_status: None,
        })))
    }
}

pub async fn get_pr_comments(
    Extension(workspace): Extension<Workspace>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<GetPrCommentsQuery>,
) -> Result<ResponseJson<ApiResponse<PrCommentsResponse, GetPrCommentsError>>, ApiError> {
    let pool = &deployment.db().pool;

    // Look up the specific repo using the multi-repo pattern
    let workspace_repo =
        WorkspaceRepo::find_by_workspace_and_repo_id(pool, workspace.id, query.repo_id)
            .await?
            .ok_or(RepoError::NotFound)?;

    let repo = Repo::find_by_id(pool, workspace_repo.repo_id)
        .await?
        .ok_or(RepoError::NotFound)?;

    // Find the merge/PR for this specific repo
    let merges = Merge::find_by_workspace_and_repo_id(pool, workspace.id, query.repo_id).await?;

    // Ensure there's an attached PR/MR for this repo
    let pr_info = match merges.into_iter().next() {
        Some(Merge::Pr(pr_merge)) => pr_merge.pr_info,
        _ => {
            return Ok(ResponseJson(ApiResponse::error_with_data(
                GetPrCommentsError::NoPrAttached,
            )));
        }
    };

    // Detect provider and create appropriate service
    let provider = git_provider::create_provider(&repo.path)
        .map_err(|e| ApiError::GitService(GitServiceError::InvalidRepository(e.to_string())))?;
    let (_, repo_id) = git_provider::detect_provider(&repo.path)
        .map_err(|e| ApiError::GitService(GitServiceError::InvalidRepository(e.to_string())))?;

    // Fetch comments from provider
    match provider
        .get_comments(&repo_id, pr_info.number as u64)
        .await
    {
        Ok(comments) => Ok(ResponseJson(ApiResponse::success(PrCommentsResponse {
            comments,
        }))),
        Err(e) => {
            tracing::error!(
                "Failed to fetch MR/PR comments for attempt {}, number #{}: {}",
                workspace.id,
                pr_info.number,
                e
            );
            match &e {
                ProviderError::NotInstalled { .. } => Ok(ResponseJson(
                    ApiResponse::error_with_data(GetPrCommentsError::GithubCliNotInstalled),
                )),
                ProviderError::NotAuthenticated(_) => Ok(ResponseJson(
                    ApiResponse::error_with_data(GetPrCommentsError::GithubCliNotLoggedIn),
                )),
                _ => Err(ApiError::GitService(GitServiceError::InvalidRepository(e.to_string()))),
            }
        }
    }
}
