import React, { FunctionComponent } from 'react';
import clsx from 'clsx';
import { useSelector } from 'react-redux';
import { AppState } from '../store/types';
import { Window } from '../store/types/model';
import { getWindow } from '../store/selectors/model';
import Control from './control';

type WindowContentProps = {
  windowId: string;
  scale?: number;
};

const WindowContent: FunctionComponent<WindowContentProps> = ({ windowId, scale = 1 }) => {
  const { window } = useConnect(windowId);
  return (
    <div style={getStyle(window, scale)} className={clsx('mylife-window-container', ...window.style)}>
      <img src={getBackground(window)} />
      {window.controls.map((control) => (
        <Control
          key={control.id}
          windowId={window.id}
          controlId={control.id}
        />
      ))}
    </div>
  );
};

export default WindowContent;

function useConnect(windowId: string) {
  return {
    window: useSelector((state: AppState) => getWindow(state, windowId))
  };
}

function getStyle(window: Window, scale: number) {
  const { height, width } = window;
  return { height, width, transform: `scale(${scale})`, transformOrigin: 'center top' };
}

function getBackground(window: Window) {
  if(!window.backgroundResource) {
    return undefined;
  }

  return `resources/${window.backgroundResource}`;
}
