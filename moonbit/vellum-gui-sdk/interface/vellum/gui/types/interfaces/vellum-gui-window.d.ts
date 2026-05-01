/** @module Interface vellum:gui/window@0.1.0 **/
export function createWindow(options: WindowOptions): number | undefined;
export function closeWindow(id: number): void;
export function setTitle(id: number, title: string): void;
export function getTitle(id: number): string;
export function minimize(id: number): void;
export function maximize(id: number): void;
export function unmaximize(id: number): void;
export function isMaximized(id: number): boolean;
export function restore(id: number): void;
export function show(id: number): void;
export function hide(id: number): void;
export function isVisible(id: number): boolean;
export function setSize(id: number, width: number, height: number): void;
export function getSize(id: number): Size;
export function setPosition(id: number, position: Point): void;
export function getPosition(id: number): Point;
export function setFullscreen(id: number, fullscreen: boolean): void;
export function isFullscreen(id: number): boolean;
export function setAlwaysOnTop(id: number, alwaysOnTop: boolean): void;
export function setCursor(id: number, shape: CursorShape): void;
export function setCursorPosition(id: number, position: Point): void;
export function requestUserAttention(id: number): void;
export function startDragging(id: number): void;
export type Color = import('./vellum-gui-types.js').Color;
export type CursorShape = import('./vellum-gui-types.js').CursorShape;
export type Size = import('./vellum-gui-types.js').Size;
export type Point = import('./vellum-gui-types.js').Point;
export interface WindowOptions {
  title: string,
  width: number,
  height: number,
  minWidth: number,
  minHeight: number,
  maxWidth: number,
  maxHeight: number,
  resizable: boolean,
  decorated: boolean,
  transparent: boolean,
  alwaysOnTop: boolean,
  center: boolean,
}
