import React, { FunctionComponent, useCallback } from 'react';
import { useDispatch } from 'react-redux';
import { viewClose } from '../store/actions/view';
import WindowContent from './window-content';
import Overlay from './overlay';
import { AppThunkDispatch } from '../store/types';

type PopupProps = {
  windowId: string;
  scale: number;
};

const Popup: FunctionComponent<PopupProps> = ({ windowId, scale }) => {
  const { onWindowClose } = useConnect();
  return (
    <>
      <Overlay onClick={onWindowClose} />
      <div
        className='mylife-window-popup'
        style={{
          WebkitTransform: `translate(-50%, -50%) scale(${scale})`,
          transform: `translate(-50%, -50%) scale(${scale})`
        }}
      >
        <WindowContent windowId={windowId} />
      </div>
    </>
  );
};

export default Popup;

function useConnect() {
  const dispatch = useDispatch<AppThunkDispatch>();

  const onWindowClose = useCallback(() => {
    dispatch(viewClose());
  }, [dispatch]);

  return { onWindowClose };
};
