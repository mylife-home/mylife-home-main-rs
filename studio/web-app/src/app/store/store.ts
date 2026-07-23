import { applyMiddleware, createStore } from 'redux';
import { createLogger } from 'redux-logger';
import { createEpicMiddleware } from 'redux-observable';
import { asyncActionMiddleware } from './common/async-action';

import rootEpic from './epics';
import reducer from './reducer';

const epicMiddleware = createEpicMiddleware();

const middlewares = [asyncActionMiddleware, epicMiddleware];

if (import.meta.env.DEV) {
  middlewares.push(createLogger());
}

export const store = createStore(reducer, applyMiddleware(...middlewares));

epicMiddleware.run(rootEpic);
