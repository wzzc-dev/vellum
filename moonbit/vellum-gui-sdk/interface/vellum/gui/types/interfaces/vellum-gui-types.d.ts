/** @module Interface vellum:gui/types@0.1.0 **/
export interface Color {
  r: number,
  g: number,
  b: number,
  a: number,
}
export interface Point {
  x: number,
  y: number,
}
export interface Size {
  width: number,
  height: number,
}
export interface Rect {
  x: number,
  y: number,
  width: number,
  height: number,
}
export interface EdgeInsets {
  top: number,
  right: number,
  bottom: number,
  left: number,
}
/**
 * # Variants
 * 
 * ## `"start"`
 * 
 * ## `"center"`
 * 
 * ## `"end"`
 * 
 * ## `"stretch"`
 * 
 * ## `"space-between"`
 * 
 * ## `"space-around"`
 * 
 * ## `"space-evenly"`
 */
export type Alignment = 'start' | 'center' | 'end' | 'stretch' | 'space-between' | 'space-around' | 'space-evenly';
/**
 * # Variants
 * 
 * ## `"start"`
 * 
 * ## `"center"`
 * 
 * ## `"end"`
 * 
 * ## `"stretch"`
 */
export type CrossAlignment = 'start' | 'center' | 'end' | 'stretch';
/**
 * # Variants
 * 
 * ## `"default"`
 * 
 * ## `"pointer"`
 * 
 * ## `"text"`
 * 
 * ## `"wait"`
 * 
 * ## `"crosshair"`
 * 
 * ## `"progress"`
 * 
 * ## `"help"`
 * 
 * ## `"move"`
 * 
 * ## `"not-allowed"`
 * 
 * ## `"no-drop"`
 * 
 * ## `"grab"`
 * 
 * ## `"grabbing"`
 */
export type CursorShape = 'default' | 'pointer' | 'text' | 'wait' | 'crosshair' | 'progress' | 'help' | 'move' | 'not-allowed' | 'no-drop' | 'grab' | 'grabbing';
/**
 * # Variants
 * 
 * ## `"visible"`
 * 
 * ## `"hidden"`
 * 
 * ## `"scroll"`
 * 
 * ## `"auto"`
 */
export type Overflow = 'visible' | 'hidden' | 'scroll' | 'auto';
/**
 * # Variants
 * 
 * ## `"none"`
 * 
 * ## `"solid"`
 * 
 * ## `"dashed"`
 * 
 * ## `"dotted"`
 * 
 * ## `"double"`
 */
export type BorderStyle = 'none' | 'solid' | 'dashed' | 'dotted' | 'double';
export interface Border {
  width: number,
  style: BorderStyle,
  color: Color,
}
export interface BoxShadow {
  offsetX: number,
  offsetY: number,
  blurRadius: number,
  spreadRadius: number,
  color: Color,
}
/**
 * # Variants
 * 
 * ## `"no-wrap"`
 * 
 * ## `"wrap"`
 * 
 * ## `"wrap-reverse"`
 */
export type Wrap = 'no-wrap' | 'wrap' | 'wrap-reverse';
/**
 * # Variants
 * 
 * ## `"visible"`
 * 
 * ## `"hidden"`
 * 
 * ## `"collapse"`
 */
export type Visibility = 'visible' | 'hidden' | 'collapse';
export interface Transform {
  translateX: number,
  translateY: number,
  scaleX: number,
  scaleY: number,
  rotation: number,
}
export type PointerEvents = PointerEventsNone | PointerEventsAll | PointerEventsAuto;
export interface PointerEventsNone {
  tag: 'none',
}
export interface PointerEventsAll {
  tag: 'all',
}
export interface PointerEventsAuto {
  tag: 'auto',
}
export interface Widget {
  id: string,
  widgetType: string,
}
