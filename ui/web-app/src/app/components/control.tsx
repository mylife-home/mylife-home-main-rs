import React, { FunctionComponent, useCallback, useMemo } from 'react';
import clsx from 'clsx';
import { useDispatch, useSelector } from 'react-redux';
import { AppState, AppThunkDispatch } from '../store/types';
import { UIControl, makeGetUIControl } from '../store/selectors/control';
import { actionPrimary, actionSecondary } from '../store/actions/actions';
import { useClickActions } from '../behaviors/input-actions';
import { useFlashFeedback } from '../behaviors/flash-feedback';

type ControlProps = {
  windowId: string;
  controlId: string;
};

const Control: FunctionComponent<ControlProps> = ({ windowId, controlId }) => {
  const { control, onActionPrimary, onActionSecondary } = useConnect(windowId, controlId);
  const { flash: flashPrimary, flashing: flashingPrimary } = useFlashFeedback();
  const { flash: flashSecondary, flashing: flashingSecondary } = useFlashFeedback();

  const handlePrimaryAction = useCallback(() => {
    if (control.hasPrimaryAction) {
      flashPrimary();
      onActionPrimary();
    }
  }, [control, flashPrimary, onActionPrimary]);
  
  const handleSecondaryAction = useCallback(() => {
    if (control.hasSecondaryAction) {
      flashSecondary();
      onActionSecondary();
    }
  }, [control, flashSecondary, onActionSecondary]);
  
  const { active, start, stop, cancel } = useClickActions(handlePrimaryAction, handleSecondaryAction);

  return (
    <>
      {/* Screen flash overlay for visual feedback */}
      {flashingPrimary && (
        <div className="mylife-screen-flash mylife-screen-flash-primary" />
      )}
      {flashingSecondary && (
        <div className="mylife-screen-flash mylife-screen-flash-secondary" />
      )}

      <div
        style={getStyleSizePosition(control)}
        className={clsx(control.hasPrimaryAction ? 'mylife-control-button' : 'mylife-control-inactive', { active }, ...control.style)}
        onTouchStart={start}
        onTouchEnd={stop}
        onTouchCancel={cancel}
        onMouseDown={start}
        onMouseUp={stop}
        onMouseLeave={cancel}
      >
        {control.displayResource && <img src={`resources/${control.displayResource}`} />}
        {control.text && <p>{control.text}</p>}
        {control.hasSecondaryAction && (
          <div className="mylife-control-secondary-indicator" />
        )}
      </div>
    </>
  )
};

export default Control;

function getStyleSizePosition(control: UIControl) {
  const { left, top, height, width } = control;
  return { left, top, height, width };
}

function useConnect(windowId: string, controlId: string) {
  const dispatch = useDispatch<AppThunkDispatch>();
  const getUIControl = useMemo(() => makeGetUIControl(windowId, controlId), [windowId, controlId]);
  return {
    control: useSelector((state: AppState) => getUIControl(state)),
    ...useMemo(() => ({
      onActionPrimary: () => dispatch(actionPrimary(windowId, controlId)),
      onActionSecondary: () => dispatch(actionSecondary(windowId, controlId))
    }), [dispatch, windowId, controlId])
  };
};
