/** @module Interface vellum:gui/widget@0.1.0 **/
export function createWidget(widgetType: string): Widget;
export function destroyWidget(id: string): void;
export function mountWidget(id: string, parentId: string): void;
export function unmountWidget(id: string): void;
export function setWidgetLayout(id: string, layout: WidgetLayout): void;
export function getWidgetLayout(id: string): WidgetLayout;
export function setWidgetSize(id: string, width: number, height: number): void;
export function getWidgetSize(id: string): Size;
export function setWidgetPosition(id: string, x: number, y: number): void;
export function getWidgetPosition(id: string): Point;
export function setWidgetPadding(id: string, insets: EdgeInsets): void;
export function setWidgetMargin(id: string, insets: EdgeInsets): void;
export function setWidgetBorder(id: string, border: Border): void;
export function setWidgetBorderRadius(id: string, radius: number): void;
export function setWidgetBackground(id: string, color: Color): void;
export function setWidgetOpacity(id: string, opacity: number): void;
export function setWidgetVisibility(id: string, visibility: Visibility): void;
export function setWidgetZIndex(id: string, zIndex: number): void;
export function setWidgetCursor(id: string, cursor: CursorShape): void;
export function setWidgetTransform(id: string, transform: Transform): void;
export function setWidgetShadow(id: string, shadow: BoxShadow | undefined): void;
export function setWidgetPointerEvents(id: string, events: PointerEvents): void;
export function markNeedsLayout(id: string): void;
export function markNeedsPaint(id: string): void;
export function getWidgetBounds(id: string): Rect;
export function getWidgetGlobalBounds(id: string): Rect;
export function setWidgetClip(id: string, clip: boolean, bounds: Rect): void;
export function focusWidget(id: string): void;
export function blurWidget(id: string): void;
export function hasFocus(id: string): boolean;
export type Color = import('./vellum-gui-types.js').Color;
export type CursorShape = import('./vellum-gui-types.js').CursorShape;
export type Visibility = import('./vellum-gui-types.js').Visibility;
export type BoxShadow = import('./vellum-gui-types.js').BoxShadow;
export type PointerEvents = import('./vellum-gui-types.js').PointerEvents;
export type Transform = import('./vellum-gui-types.js').Transform;
export type Size = import('./vellum-gui-types.js').Size;
export type Point = import('./vellum-gui-types.js').Point;
export type Rect = import('./vellum-gui-types.js').Rect;
export type EdgeInsets = import('./vellum-gui-types.js').EdgeInsets;
export type Alignment = import('./vellum-gui-types.js').Alignment;
export type CrossAlignment = import('./vellum-gui-types.js').CrossAlignment;
export type Wrap = import('./vellum-gui-types.js').Wrap;
export type Border = import('./vellum-gui-types.js').Border;
export type Overflow = import('./vellum-gui-types.js').Overflow;
export type Widget = import('./vellum-gui-types.js').Widget;
/**
 * # Variants
 * 
 * ## `"none"`
 * 
 * ## `"flex"`
 * 
 * ## `"block"`
 * 
 * ## `"inline"`
 * 
 * ## `"inline-block"`
 * 
 * ## `"inline-flex"`
 * 
 * ## `"grid"`
 */
export type WidgetDisplay = 'none' | 'flex' | 'block' | 'inline' | 'inline-block' | 'inline-flex' | 'grid';
/**
 * # Variants
 * 
 * ## `"row"`
 * 
 * ## `"row-reverse"`
 * 
 * ## `"column"`
 * 
 * ## `"column-reverse"`
 */
export type FlexDirection = 'row' | 'row-reverse' | 'column' | 'column-reverse';
/**
 * # Variants
 * 
 * ## `"static-position"`
 * 
 * ## `"relative"`
 * 
 * ## `"absolute"`
 * 
 * ## `"fixed"`
 * 
 * ## `"sticky"`
 */
export type WidgetPosition = 'static-position' | 'relative' | 'absolute' | 'fixed' | 'sticky';
export interface WidgetLayout {
  display: WidgetDisplay,
  flexDirection: FlexDirection,
  flexWrap: Wrap,
  justifyContent: Alignment,
  alignItems: CrossAlignment,
  alignContent: Alignment,
  gap: number,
  rowGap: number,
  columnGap: number,
  position: WidgetPosition,
  minWidth?: number,
  maxWidth?: number,
  minHeight?: number,
  maxHeight?: number,
  overflowX: Overflow,
  overflowY: Overflow,
}
