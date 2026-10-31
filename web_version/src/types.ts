/** Rust 负责计算，网页只编辑输入及展示这些传输对象。 */
export interface Tool {
  id: string;
  name: string;
  group: string;
  icon: string;
  description: string;
  kind: string;
  rememberInputs: boolean;
  note: string;
  formula: string;
  noteModes: { mode: number; note: string; formula: string }[];
  units: { name: string; code: string }[] | null;
}
export interface Catalog {
  tools: Tool[];
  rtd: { key: string; name: string; r0: number; min: number; max: number }[];
  zones: {
    min: number;
    max: number;
    coefficients: number[];
    points: string[][];
  }[][];
  lowRanges: { label: string; min: number; fixed: number[] }[];
  defaults: {
    ntcA: string[];
    ntcB: string[];
    ntcPoints: string[][];
    sprtFixed: Record<string, string>;
  };
}
export type State = Record<string, unknown>;
export interface Result {
  metrics: { label: string; value: unknown; unit: string }[];
  headings: string[];
  rows: unknown[][];
  curve: [number, number][];
  points: [number, number][];
  residuals: [number, number][];
  coefficients: number[];
  tolerances: Tolerance[];
  warnings: string[];
  details: Record<string, unknown> | null;
  total: number;
  equation: string;
}
export interface Tolerance {
  name: string;
  formula: string;
  min: number;
  max: number;
  delta: number | null;
  unit: string;
  temperature: [number, number] | null;
  signal: [number, number] | null;
  signalUnit: string;
}
export interface Field {
  key: string;
  label: string;
  type:
    | "number"
    | "text"
    | "select"
    | "vector"
    | "points"
    | "budget"
    | "pressure";
  default: unknown;
  options?: { label: string; value: string }[];
  min?: number;
  max?: number;
  labels?: string[];
  hint?: string;
  locked?: number[];
  budgetKinds?: string[];
}
export interface Section {
  title: string;
  fields: Field[];
  advanced?: { title: string; fields: Field[] };
}
export interface Budget {
  name: string;
  kind: string;
  sensitivity: string;
  values: string[];
  method: string;
  halfWidth: string;
  distribution: string;
  combine: string;
  children: Budget[];
}
export interface BudgetDetail {
  name: string;
  standard: number;
  sensitivity: number;
  contribution: number;
  children: BudgetDetail[];
}
