import React, { FunctionComponent } from 'react';
import Overlay from './overlay';

const Loading: FunctionComponent = () => (
  <Overlay>
    <img src='loading.svg' className='mylife-img-loading' />
  </Overlay>
);

export default Loading;