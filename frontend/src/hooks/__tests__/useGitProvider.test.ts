/**
 * Unit tests for useGitProvider hook
 *
 * Tests provider detection and terminology selection
 */

import { describe, it, expect } from 'vitest';
import {
  useGitProvider,
  getProviderType,
  getProviderTerminology,
  type ProviderType,
} from '../useGitProvider';
import { renderHook } from '@testing-library/react';
import type { Workspace } from 'shared/types';

describe('useGitProvider', () => {
  it('returns GitHub by default when no provider specified', () => {
    const workspace: Partial<Workspace> = {
      id: 'test-workspace',
      name: 'Test Workspace',
    };

    const { result } = renderHook(() => useGitProvider(workspace as Workspace));

    expect(result.current.provider).toBe('github');
    expect(result.current.terminology.pr).toBe('Pull Request');
    expect(result.current.terminology.prShort).toBe('PR');
    expect(result.current.terminology.cli).toBe('GitHub CLI');
    expect(result.current.terminology.cliCommand).toBe('gh');
  });

  it('returns GitHub for github.com remotes', () => {
    const workspace: Partial<Workspace> = {
      id: 'test-workspace',
      name: 'Test Workspace',
      git_provider: 'github',
    };

    const { result } = renderHook(() => useGitProvider(workspace as Workspace));

    expect(result.current.provider).toBe('github');
    expect(result.current.terminology.pr).toBe('Pull Request');
  });

  it('returns GitLab for gitlab.com remotes', () => {
    const workspace: Partial<Workspace> = {
      id: 'test-workspace',
      name: 'Test Workspace',
      git_provider: 'gitlab',
    };

    const { result } = renderHook(() => useGitProvider(workspace as Workspace));

    expect(result.current.provider).toBe('gitlab');
    expect(result.current.terminology.pr).toBe('Merge Request');
    expect(result.current.terminology.prShort).toBe('MR');
    expect(result.current.terminology.cli).toBe('GitLab CLI');
    expect(result.current.terminology.cliCommand).toBe('glab');
  });

  it('is case-insensitive for provider detection', () => {
    const workspace1: Partial<Workspace> = {
      id: 'test-1',
      git_provider: 'GITLAB',
    };
    const workspace2: Partial<Workspace> = {
      id: 'test-2',
      git_provider: 'GitLab',
    };

    const { result: result1 } = renderHook(() => useGitProvider(workspace1 as Workspace));
    const { result: result2 } = renderHook(() => useGitProvider(workspace2 as Workspace));

    expect(result1.current.provider).toBe('gitlab');
    expect(result2.current.provider).toBe('gitlab');
  });

  it('handles null workspace', () => {
    const { result } = renderHook(() => useGitProvider(null));

    expect(result.current.provider).toBe('github'); // defaults to GitHub
  });

  it('handles undefined workspace', () => {
    const { result } = renderHook(() => useGitProvider(undefined));

    expect(result.current.provider).toBe('github'); // defaults to GitHub
  });
});

describe('getProviderType', () => {
  it('returns github for GitHub workspace', () => {
    const workspace: Partial<Workspace> = {
      git_provider: 'github',
    };

    expect(getProviderType(workspace as Workspace)).toBe('github');
  });

  it('returns gitlab for GitLab workspace', () => {
    const workspace: Partial<Workspace> = {
      git_provider: 'gitlab',
    };

    expect(getProviderType(workspace as Workspace)).toBe('gitlab');
  });

  it('returns github by default', () => {
    const workspace: Partial<Workspace> = {
      git_provider: undefined,
    };

    expect(getProviderType(workspace as Workspace)).toBe('github');
  });

  it('handles null workspace', () => {
    expect(getProviderType(null)).toBe('github');
  });
});

describe('getProviderTerminology', () => {
  it('returns correct GitHub terminology', () => {
    const terminology = getProviderTerminology('github');

    expect(terminology.pr).toBe('Pull Request');
    expect(terminology.prShort).toBe('PR');
    expect(terminology.cli).toBe('GitHub CLI');
    expect(terminology.cliCommand).toBe('gh');
  });

  it('returns correct GitLab terminology', () => {
    const terminology = getProviderTerminology('gitlab');

    expect(terminology.pr).toBe('Merge Request');
    expect(terminology.prShort).toBe('MR');
    expect(terminology.cli).toBe('GitLab CLI');
    expect(terminology.cliCommand).toBe('glab');
  });

  it('terminology is consistent between hook and function', () => {
    const workspace: Partial<Workspace> = {
      git_provider: 'gitlab',
    };

    const { result } = renderHook(() => useGitProvider(workspace as Workspace));
    const directTerminology = getProviderTerminology('gitlab');

    expect(result.current.terminology).toEqual(directTerminology);
  });
});

describe('Provider terminology differences', () => {
  it('GitHub and GitLab have different PR terminology', () => {
    const github = getProviderTerminology('github');
    const gitlab = getProviderTerminology('gitlab');

    expect(github.pr).not.toBe(gitlab.pr);
    expect(github.prShort).not.toBe(gitlab.prShort);
  });

  it('GitHub uses "Pull Request", GitLab uses "Merge Request"', () => {
    const github = getProviderTerminology('github');
    const gitlab = getProviderTerminology('gitlab');

    expect(github.pr).toBe('Pull Request');
    expect(gitlab.pr).toBe('Merge Request');
  });

  it('CLI commands are different', () => {
    const github = getProviderTerminology('github');
    const gitlab = getProviderTerminology('gitlab');

    expect(github.cliCommand).toBe('gh');
    expect(gitlab.cliCommand).toBe('glab');
  });
});

describe('Memoization behavior', () => {
  it('returns same result for same workspace', () => {
    const workspace: Partial<Workspace> = {
      id: 'test',
      git_provider: 'gitlab',
    };

    const { result, rerender } = renderHook(() => useGitProvider(workspace as Workspace));
    const firstResult = result.current;

    rerender();

    expect(result.current).toBe(firstResult); // same reference (memoized)
  });

  it('updates when git_provider changes', () => {
    const workspace: Partial<Workspace> = {
      id: 'test',
      git_provider: 'github',
    };

    const { result, rerender } = renderHook(
      ({ ws }) => useGitProvider(ws as Workspace),
      { initialProps: { ws: workspace } }
    );

    expect(result.current.provider).toBe('github');

    // Change provider
    workspace.git_provider = 'gitlab';
    rerender({ ws: workspace });

    expect(result.current.provider).toBe('gitlab');
  });
});
