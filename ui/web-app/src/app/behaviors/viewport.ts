import { useEffect, useRef, useState } from 'react';
import { useSelector } from 'react-redux';
import { AppState } from '../store/types';
import { getWindow } from '../store/selectors/model';

export interface Size {
  readonly width: number;
  readonly height: number;
}

export function useViewport(windowId: string) {
  const { window } = useConnect(windowId);
  return useViewportScale({ width: window.width, height: window.height });
}

function useConnect(windowId: string) {
  return {
    window: useSelector((state: AppState) => getWindow(state, windowId))
  };
}

function useViewportScale(size: Size) {
  const [scale, setScale] = useState(1);
  const requiredSizeRef = useRef<Size | null>(null);

  useEffect(() => {
    window.addEventListener('orientationchange', adjustViewport);
    window.addEventListener('resize', adjustViewport);

    return () => {
      window.removeEventListener('orientationchange', adjustViewport);
      window.removeEventListener('resize', adjustViewport);
    };
  }, []);

  useEffect(() => {
    requiredSizeRef.current = size;
    adjustViewport();
  }, [size.width, size.height]);

  function adjustViewport(): void {
    if (!requiredSizeRef.current) {
      return;
    }

    const actualSize = getDisplaySize();
    const ratio = Math.min(actualSize.width / requiredSizeRef.current.width, actualSize.height / requiredSizeRef.current.height);

    console.log(`viewport scale: ${ratio} for ${actualSize.width}x${actualSize.height}`); // eslint-disable-line no-console
    setScale(ratio);
  }

  return scale;
}


function getDisplaySize(): Size {
  const { documentElement } = document;
  const width = documentElement.clientWidth || window.innerWidth;
  const height = window.innerHeight || documentElement.clientHeight;

  return { width, height };
}