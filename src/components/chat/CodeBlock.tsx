import { useState, useMemo } from "react";
import Prism from "prismjs";

import "../../lib/PrismLanguages";
import { PRISM_ALIASES } from "../../constants/PrismAliases";
import { TOKEN_STYLES, BRACKET_COLORS } from "../../constants/PrismTheme";

interface CodeBlockProps {
  lang: string;
  code: string;
}

function resolveGrammar(
  lang: string,
): { grammar: Prism.Grammar; language: string } | null {
  const normalized = lang.toLowerCase().trim();
  const resolved = PRISM_ALIASES[normalized] ?? normalized;
  const grammar = Prism.languages[resolved];
  return grammar ? { grammar, language: resolved } : null;
}

// ─── Recursive renderer ───────────────────────────────────────────────────────

function renderTokenNode(
  token: string | Prism.Token,
  key: string,
  bracketDepth: { value: number },
): React.ReactNode {
  if (typeof token === "string") {
    // Plain string — check for bracket coloring character-by-character only
    // when the entire string is a single bracket. Multi-char plain strings are
    // rendered as-is to avoid splitting text nodes unnecessarily.
    if (token.length === 1 && "([{)]}".includes(token)) {
      const isOpen = "([{".includes(token);
      const depth = isOpen ? bracketDepth.value++ : --bracketDepth.value;
      return (
        <span key={key} style={{ color: BRACKET_COLORS[Math.abs(depth) % 3] }}>
          {token}
        </span>
      );
    }
    return (
      <span key={key} style={{ color: TOKEN_STYLES["plain"] }}>
        {token}
      </span>
    );
  }

  // ── Prism.Token ──
  // token.type can be a space-separated list of types (e.g. "keyword control-flow").
  // Try the full compound key first, then the first type, then fall back to plain.
  const types = token.type.split(" ");
  const color =
    TOKEN_STYLES[token.type] ?? TOKEN_STYLES[types[0]] ?? TOKEN_STYLES["plain"];

  const content = token.content;

  if (typeof content === "string") {
    if (content.length === 1 && "([{)]}".includes(content)) {
      const isOpen = "([{".includes(content);
      const depth = isOpen ? bracketDepth.value++ : --bracketDepth.value;
      return (
        <span key={key} style={{ color: BRACKET_COLORS[Math.abs(depth) % 3] }}>
          {content}
        </span>
      );
    }
    return (
      <span key={key} style={{ color }}>
        {content}
      </span>
    );
  }

  if (Array.isArray(content)) {
    return (
      <span key={key} style={{ color }}>
        {content.map((child, i) =>
          renderTokenNode(child, `${key}-${i}`, bracketDepth),
        )}
      </span>
    );
  }

  // Nested Token (Prism.Token whose content is another Token — rare but valid)
  if (content && typeof content === "object") {
    return (
      <span key={key} style={{ color }}>
        {renderTokenNode(content as Prism.Token, `${key}-0`, bracketDepth)}
      </span>
    );
  }

  return null;
}

function renderTokens(tokens: (string | Prism.Token)[]): React.ReactNode {
  const bracketDepth = { value: 0 };
  return tokens.map((token, i) =>
    renderTokenNode(token, String(i), bracketDepth),
  );
}

// ─── Component ────────────────────────────────────────────────────────────────

export function CodeBlock({ lang, code }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard?.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const highlighted = useMemo(() => {
    const resolved = resolveGrammar(lang);
    if (!resolved) {
      return <span style={{ color: TOKEN_STYLES["plain"] }}>{code}</span>;
    }
    const tokens = Prism.tokenize(code, resolved.grammar);
    return renderTokens(tokens);
  }, [lang, code]);

  return (
    <div className="my-2.5 overflow-hidden rounded-md bg-slate-grey-950 border border-slate-grey-800">
      <div className="flex justify-between items-center px-3 py-1.25 bg-slate-grey-900 border-b border-slate-grey-800">
        <span className="font-mono text-[11px] tracking-wider text-warm-grey-600">
          {lang}
        </span>
        <button
          onClick={handleCopy}
          className={`font-mono bg-transparent border rounded-sm px-2 py-0.5 text-[11px] cursor-pointer transition-colors duration-200 ${
            copied
              ? "text-indigo-smoke-400 border-indigo-smoke-700"
              : "text-warm-grey-600 border-slate-grey-800 hover:text-warm-grey-400"
          }`}
        >
          {copied ? "copied!" : "copy"}
        </button>
      </div>
      <pre className="m-0 px-4 py-3.5 overflow-x-auto font-mono text-[13px] leading-[1.65]">
        <code>{highlighted}</code>
      </pre>
    </div>
  );
}
