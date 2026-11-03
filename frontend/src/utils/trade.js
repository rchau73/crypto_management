// "Record trade": the pure part of the dialog — which exchanges a position
// is held on, the pre-filled numbers, the preview of what will change and
// the request sent. The server checks everything again; this only keeps
// the form honest before it is sent.
import { CASH_BARCA } from "./rebalance";

export const NEW_SOURCE = "__new__";

// Blank and missing notes are the same, unnamed source (as on the server).
export function sourceName(notes) {
  return (notes || "").trim();
}

export function sourceLabel(notes) {
  return sourceName(notes) || "No exchange named";
}

// A stable id per source for the pickers; biggest first, so the default
// pick is where most of the position sits.
function withIds(sources) {
  return sources
    .map((s) => ({ ...s, id: [s.symbol, s.group_name || "", s.barca || "", s.asset_class, sourceName(s.notes)].join("|") }))
    .sort((a, b) => b.current_quantity - a.current_quantity || sourceName(a.notes).localeCompare(sourceName(b.notes)));
}

// The sources of `symbol` inside `barca`, biggest first.
export function assetSources(sources, symbol, barca) {
  return withIds(sources.filter((s) => s.symbol === symbol && (s.barca || "") === (barca || "")));
}

// Every Caixa source (stablecoins, cash), biggest first.
export function cashSources(sources) {
  return withIds(sources.filter((s) => s.barca === CASH_BARCA));
}

// The cash source on the same exchange as the trade, else the biggest one.
export function defaultCashSource(cashOptions, exchange) {
  const wanted = sourceName(exchange).toLowerCase();
  return cashOptions.find((c) => wanted && sourceName(c.notes).toLowerCase() === wanted) ?? cashOptions[0] ?? null;
}

// Every position the dialog can trade, as "symbol in barca", without Caixa
// (it is the cash side, not a trade).
export function tradablePositions(allocations) {
  const seen = new Map();
  allocations
    .filter((a) => a.barca !== CASH_BARCA)
    .forEach((a) => seen.set(`${a.symbol}|${a.barca}`, { symbol: a.symbol, barca: a.barca }));
  return Array.from(seen.values()).sort((a, b) => a.symbol.localeCompare(b.symbol) || a.barca.localeCompare(b.barca));
}

export function priceOf(allocations, symbol) {
  const row = allocations.find((a) => a.symbol === symbol && typeof a.price === "number" && a.price > 0);
  return row ? row.price : null;
}

// A typed number > 0, or null.
export function parsePositive(text) {
  if (typeof text !== "string" || text.trim() === "") return null;
  const n = Number(text.trim().replace(",", "."));
  return Number.isFinite(n) && n > 0 ? n : null;
}

// Up to 8 decimals without trailing zeros: crypto quantities are small.
export function formatQuantity(n) {
  if (typeof n !== "number" || !Number.isFinite(n)) return "-";
  return n.toLocaleString("en-US", { maximumFractionDigits: 8 });
}

// A pre-filled input value: rounded to 8 decimals, no exponent.
export function inputNumber(n) {
  if (typeof n !== "number" || !Number.isFinite(n) || n <= 0) return "";
  return String(Number(n.toFixed(8)));
}

// The preview and the problems to fix before sending. `held`/`cashHeld`
// are the chosen sources' current quantities (0 for a new exchange).
export function previewTrade({ side, held, quantity, price, useCash, cashHeld, cashQuantity }) {
  const sign = side === "buy" ? 1 : -1;
  const problems = [];
  if (quantity === null) problems.push("Enter a quantity above 0.");
  if (price === null) problems.push("Enter a price above 0.");
  const newHeld = quantity === null ? null : held + sign * quantity;
  if (newHeld !== null && newHeld < -1e-9) problems.push(`That exchange holds only ${formatQuantity(held)}; you can't sell more.`);
  let newCash = null;
  if (useCash) {
    // Until there's a total, the missing quantity/price is the problem.
    if (cashQuantity === null) {
      if (quantity !== null && price !== null) problems.push("Enter the cash moved, above 0.");
    }
    else {
      newCash = cashHeld - sign * cashQuantity;
      if (newCash < -1e-9) problems.push(`The cash source holds only ${formatQuantity(cashHeld)}.`);
    }
  }
  return {
    total: quantity !== null && price !== null ? quantity * price : null,
    newHeld: newHeld === null ? null : Math.max(0, newHeld),
    newCash: newCash === null ? null : Math.max(0, newCash),
    problems,
  };
}

function ref(source, exchange) {
  return {
    symbol: source.symbol,
    group_name: source.group_name ?? null,
    barca: source.barca ?? null,
    asset_class: source.asset_class,
    source: sourceName(exchange) || null,
  };
}

// The POST /trades body. `asset`: the chosen source (or, for a new
// exchange, any source of the position plus `exchange`).
export function buildTradeRequest({ side, asset, exchange, quantity, price, cash, cashQuantity }) {
  return {
    side,
    asset: ref(asset, exchange),
    quantity,
    price,
    cash: cash ? { position: ref(cash, cash.notes), quantity: cashQuantity } : null,
  };
}
