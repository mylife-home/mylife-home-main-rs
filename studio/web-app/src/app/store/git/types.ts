import type { GitStatus, GitDiff, GitDiffFile, GitDiffChunk, GitDiffChange } from '../../api/git';
import { Table } from '../common/types';
export type { GitStatus, GitDiff, GitDiffFile, GitDiffChunk, GitDiffChange };

export const DEFAULT_STATUS: GitStatus = {
  appUrl: null,
  branch: '<unknown>',
  changedFeatures: [],
  ahead: null,
  behind: null,
};

export const enum ActionTypes {
  SET_NOTIFICATION = 'git/set-notification',
  CLEAR_NOTIFICATION = 'git/clear-notification',
  SET_STATUS = 'git/set-status',
  REFRESH = 'git/refresh',
  COMMIT = 'git/commit',
  RESTORE = 'git/restore',
  DIFF = 'git/diff',
  DIFF_DATA_SET = 'git/diff-data-set',
  DIFF_DATA_CLEAR = 'git/diff-data-clear',
  DIFF_STAGE = 'git/diff-stage',
}

export namespace ActionPayloads {
  export type SetNotification = string;
  export type ClearNotification = void;
  export type SetStatus = GitStatus;
  export type GitCommit = { message: string };
  export type GitRestore = { type: 'all' | 'feature' | 'file'; id?: string; };
  export type GitDiffDataSet = GitDiff;
  export type GitDiffDataClear = void;
  export type GitDiffStage = { type: 'all' | 'feature' | 'file'; id?: string; stage: boolean };
}

export interface Feature {
  id: string;
  files: string[];
}

export interface File extends Omit<GitDiffFile, 'chunks' | 'feature'> {
  id: string;
  name: string;
  chunks: string[];
  staged: boolean;
}

export interface Chunk extends GitDiffChunk {
  id: string;
}

export interface GitState {
  notifierId: string;
  status: GitStatus;

  // filled when we have diff data
  features: Table<Feature>;
  files: Table<File>;
  chunks: Table<Chunk>;
}