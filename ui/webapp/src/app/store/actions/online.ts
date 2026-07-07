import { createAction } from '@reduxjs/toolkit';

export const onlineSet = createAction<boolean>('online/set');
