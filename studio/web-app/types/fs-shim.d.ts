// csv-writer-browser use "fs.writeFile" internally.

declare module 'fs' {
  export function writeFile(...args: any[]): void;
}