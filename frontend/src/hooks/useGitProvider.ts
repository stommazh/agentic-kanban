import { useMemo } from 'react';
import type { Workspace } from 'shared/types';

export type ProviderType = 'github' | 'gitlab';

export interface ProviderTerminology {
  /** Full term: "Pull Request" or "Merge Request" */
  pr: string;
  /** Short term: "PR" or "MR" */
  prShort: string;
  /** CLI tool name: "GitHub CLI" or "GitLab CLI" */
  cli: string;
  /** CLI command: "gh" or "glab" */
  cliCommand: string;
}

export interface GitProviderInfo {
  provider: ProviderType;
  terminology: ProviderTerminology;
}

/**
 * Hook to detect git provider for a workspace and get appropriate terminology.
 * Provider can be explicitly cached in workspace.git_provider or defaults to GitHub.
 *
 * @param workspace - The workspace object containing git_provider field
 * @returns Provider type and localized terminology
 */
export function useGitProvider(workspace: Workspace | null | undefined): GitProviderInfo {
  return useMemo(() => {
    // Use cached provider from workspace if available
    const providerStr = workspace?.git_provider?.toLowerCase();
    const provider: ProviderType =
      providerStr === 'gitlab' ? 'gitlab' : 'github';

    const terminology: ProviderTerminology =
      provider === 'gitlab'
        ? {
            pr: 'Merge Request',
            prShort: 'MR',
            cli: 'GitLab CLI',
            cliCommand: 'glab',
          }
        : {
            pr: 'Pull Request',
            prShort: 'PR',
            cli: 'GitHub CLI',
            cliCommand: 'gh',
          };

    return { provider, terminology };
  }, [workspace?.git_provider]);
}

/**
 * Detect provider type from workspace
 */
export function getProviderType(workspace: Workspace | null | undefined): ProviderType {
  const providerStr = workspace?.git_provider?.toLowerCase();
  return providerStr === 'gitlab' ? 'gitlab' : 'github';
}

/**
 * Get terminology for a specific provider type
 */
export function getProviderTerminology(provider: ProviderType): ProviderTerminology {
  return provider === 'gitlab'
    ? {
        pr: 'Merge Request',
        prShort: 'MR',
        cli: 'GitLab CLI',
        cliCommand: 'glab',
      }
    : {
        pr: 'Pull Request',
        prShort: 'PR',
        cli: 'GitHub CLI',
        cliCommand: 'gh',
      };
}
