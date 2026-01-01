import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { defineModal, getErrorMessage } from '@/lib/modals';
import type { GhCliSetupError } from 'shared/types';
import { useRef, useState } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface GlabCliSetupDialogProps {
  attemptId: string;
}

export type GlabCliSupportVariant = 'homebrew' | 'manual';

export interface GlabCliSupportContent {
  message: string;
  variant: GlabCliSupportVariant | null;
}

export const mapGlabCliErrorToUi = (
  error: GhCliSetupError | null,
  fallbackMessage: string,
  t: (key: string) => string
): GlabCliSupportContent => {
  if (!error) {
    return { message: fallbackMessage, variant: null };
  }

  if (error === 'BREW_MISSING') {
    return {
      message: t('settings:integrations.gitlab.cliSetup.errors.brewMissing'),
      variant: 'homebrew',
    };
  }

  if (error === 'SETUP_HELPER_NOT_SUPPORTED') {
    return {
      message: t('settings:integrations.gitlab.cliSetup.errors.notSupported'),
      variant: 'manual',
    };
  }

  if (typeof error === 'object' && 'OTHER' in error) {
    return {
      message: error.OTHER.message || fallbackMessage,
      variant: null,
    };
  }

  return { message: fallbackMessage, variant: null };
};

export const GlabCliHelpInstructions = ({
  variant,
  t,
}: {
  variant: GlabCliSupportVariant;
  t: (key: string) => string;
}) => {
  if (variant === 'homebrew') {
    return (
      <div className="space-y-2 text-sm">
        <p>
          {t('settings:integrations.gitlab.cliSetup.help.homebrew.description')}{' '}
          <a
            href="https://brew.sh/"
            target="_blank"
            rel="noreferrer"
            className="underline"
          >
            {t('settings:integrations.gitlab.cliSetup.help.homebrew.brewSh')}
          </a>{' '}
          {t(
            'settings:integrations.gitlab.cliSetup.help.homebrew.manualInstall'
          )}
        </p>
        <pre className="rounded bg-muted px-2 py-1 text-xs">
          brew install glab
        </pre>
        <p>
          {t(
            'settings:integrations.gitlab.cliSetup.help.homebrew.afterInstall'
          )}
          <br />
          <code className="rounded bg-muted px-1 py-0.5 text-xs">
            glab auth login
          </code>
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-2 text-sm">
      <p>
        {t('settings:integrations.gitlab.cliSetup.help.manual.description')}{' '}
        <a
          href="https://gitlab.com/gitlab-org/cli"
          target="_blank"
          rel="noreferrer"
          className="underline"
        >
          {t('settings:integrations.gitlab.cliSetup.help.manual.officialDocs')}
        </a>{' '}
        {t('settings:integrations.gitlab.cliSetup.help.manual.andAuthenticate')}
      </p>
      <pre className="rounded bg-muted px-2 py-1 text-xs">
        glab auth login
      </pre>
    </div>
  );
};

const GlabCliSetupDialogImpl = NiceModal.create<GlabCliSetupDialogProps>(
  () => {
    const modal = useModal();
    const { t } = useTranslation();
    const [isRunning, setIsRunning] = useState(false);
    const [errorInfo, setErrorInfo] = useState<{
      error: GhCliSetupError;
      message: string;
      variant: GlabCliSupportVariant | null;
    } | null>(null);
    const pendingResultRef = useRef<GhCliSetupError | null>(null);
    const hasResolvedRef = useRef(false);

    const handleRunSetup = async () => {
      setIsRunning(true);
      setErrorInfo(null);
      pendingResultRef.current = null;

      try {
        // TODO: Implement GitLab CLI setup endpoint when backend support is added
        // await attemptsApi.setupGlabCli(attemptId);
        throw new Error('GitLab CLI setup not yet implemented');
      } catch (err: unknown) {
        const rawMessage =
          getErrorMessage(err) ||
          t('settings:integrations.gitlab.cliSetup.errors.setupFailed');

        let errorData: GhCliSetupError | null = null;
        if (err && typeof err === 'object' && 'error_data' in err) {
          errorData = (err as { error_data?: GhCliSetupError }).error_data ?? null;
        }

        const { message, variant } = mapGlabCliErrorToUi(
          errorData,
          rawMessage,
          t
        );

        pendingResultRef.current = errorData;
        setErrorInfo({ error: errorData || 'SETUP_HELPER_NOT_SUPPORTED', message, variant });
      } finally {
        setIsRunning(false);
      }
    };

    const handleClose = () => {
      if (!hasResolvedRef.current) {
        modal.resolve(pendingResultRef.current || 'SETUP_HELPER_NOT_SUPPORTED');
      }
      modal.hide();
    };

    return (
      <Dialog open={modal.visible} onOpenChange={handleClose}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t('settings:integrations.gitlab.cliSetup.title')}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              {t('settings:integrations.gitlab.cliSetup.description')}
            </p>
            {errorInfo && (
              <Alert variant={errorInfo.variant ? 'default' : 'destructive'}>
                <AlertDescription className="space-y-3">
                  <p>{errorInfo.message}</p>
                  {errorInfo.variant && (
                    <GlabCliHelpInstructions variant={errorInfo.variant} t={t} />
                  )}
                </AlertDescription>
              </Alert>
            )}
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={handleClose}>
              {t('common:buttons.cancel')}
            </Button>
            <Button onClick={handleRunSetup} disabled={isRunning}>
              {isRunning ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('settings:integrations.gitlab.cliSetup.running')}
                </>
              ) : (
                t('settings:integrations.gitlab.cliSetup.run')
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const GlabCliSetupDialog = defineModal<
  GlabCliSetupDialogProps,
  GhCliSetupError | null
>(GlabCliSetupDialogImpl);
