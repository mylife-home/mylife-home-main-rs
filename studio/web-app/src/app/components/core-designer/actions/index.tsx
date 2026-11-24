import React, { FunctionComponent } from 'react';
import Toolbar from '@material-ui/core/Toolbar';
import CheckIcon from '@material-ui/icons/Check';
import PublishIcon from '@material-ui/icons/Publish';
import GetAppIcon from '@material-ui/icons/GetApp';

import { ToolbarIconButton, IconWithBadge } from '../../lib/toolbar';
import { InstanceIcon, ProjectIcon } from '../../lib/icons';
import { useImportFromProject, useImportFromOnline, useProjectDeploy, useProjectValidation } from './behaviors';

const Actions: FunctionComponent<{ className?: string }> = ({ className }) => {
  const importFromProject = useImportFromProject();
  const ImportFromOnline = useImportFromOnline();
  const validateProject = useProjectValidation();
  const deployProject = useProjectDeploy();

  return (
    <Toolbar className={className}>
      <ToolbarIconButton title="Importer depuis un autre projet" icon={<ImportFromProject />} onClick={importFromProject} />
      <ToolbarIconButton title="Importer depuis les entités en ligne" icon={<ImoprtFromOnlineIcon />} onClick={ImportFromOnline} />
      <ToolbarIconButton title="Valider par rapport aux instances en ligne" icon={<CheckIcon />} onClick={validateProject} />
      <ToolbarIconButton title="Déployer" icon={<PublishIcon />} onClick={deployProject} />
    </Toolbar>
  );
}

export default Actions;

const ImportFromProject: FunctionComponent = () => (
  <IconWithBadge
    main={<GetAppIcon />}
    badge={<ProjectIcon />}
  />
);

const ImoprtFromOnlineIcon: FunctionComponent = () => (
  <IconWithBadge
    main={<GetAppIcon />}
    badge={<InstanceIcon />}
  />
);
