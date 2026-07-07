import { applyMiddleware, createStore } from 'redux';
import thunk from 'redux-thunk';
import { createLogger } from 'redux-logger';

import { socketMiddleware } from './middlewares/socket';
import { cssMiddleware } from './middlewares/css';
import { resourcesMiddleware } from './middlewares/resources';
import { navigationMiddleware } from './middlewares/navigation';
import reducer from './reducers/index';

const middlewares = [navigationMiddleware, socketMiddleware, resourcesMiddleware, cssMiddleware, thunk];

if (process.env.NODE_ENV !== 'production') {
  middlewares.push(createLogger());
}

export const store = createStore(reducer, applyMiddleware(...middlewares));
