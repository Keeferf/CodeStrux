export const BRACKET_COLORS = ["#e8c97e", "#c792ea", "#89ddff"] as const;

// ─── Token color theme (Material Palenight-inspired) ─────────────────────────
// Keys map directly to Prism token types returned by Prism.tokenize().

export const TOKEN_STYLES: Record<string, string> = {
  // Keywords & control flow
  keyword: "#c792ea",
  "keyword control-flow": "#c792ea",

  // Strings & template literals
  string: "#c3e88d",
  "template-string": "#c3e88d",
  "template-punctuation": "#c3e88d",

  // Comments
  comment: "#546e7a",
  prolog: "#546e7a",
  doctype: "#546e7a",

  // Numbers & booleans
  number: "#f78c6c",
  boolean: "#f78c6c",

  // Functions & methods
  function: "#82aaff",
  "function-variable": "#82aaff",
  method: "#82aaff",

  // Classes & types
  "class-name": "#ffcb6b",
  builtin: "#ffcb6b",

  // HTML / JSX tags
  tag: "#f07178",
  "tag .punctuation": "#f07178",

  // Attributes
  "attr-name": "#ffcb6b",
  "attr-value": "#c3e88d",

  // Punctuation & operators
  punctuation: "#89ddff",
  operator: "#89ddff",
  arrow: "#89ddff",

  // Variables, parameters & constants
  parameter: "#f07178",
  variable: "#d4cfc9",
  constant: "#f78c6c",

  // Regex
  regex: "#f07178",

  // Imports / modules / namespaces
  module: "#c792ea",
  imports: "#d4cfc9",
  namespace: "#d4cfc9",

  // CSS-specific
  property: "#89ddff",
  selector: "#c3e88d",
  unit: "#f78c6c",
  atrule: "#c792ea",

  // Misc
  important: "#f78c6c",
  italic: "#89ddff",
  bold: "#c3e88d",

  // Fallback plain text
  plain: "#d4cfc9",
};
