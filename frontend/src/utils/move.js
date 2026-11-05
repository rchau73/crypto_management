// "Move balance": the pure part of the dialog — moving a quantity of one
// asset to another BARCA/group and/or exchange (e.g. 0.5 BTC from Base on
// Binance to RendaPassiva on Deribit). No price and no trade: the value
// only changes place, less any fee. The server checks everything again.
import { formatQuantity, sourceName, sourceOptions } from "./trade";

export const QUICK_PERCENTS = [10, 15, 25, 50, 100];

// Names compare like the server does: surrounding spaces and capitals
// don't make a different group, BARCA or exchange ("binance" is "Binance").
function sameName(a, b) {
  return (a || "").trim().toLowerCase() === (b || "").trim().toLowerCase();
}

function uniqueSorted(values) {
  return Array.from(new Set(values.filter(Boolean))).sort((a, b) => a.localeCompare(b));
}

// Assets held somewhere, so there is something to move.
export function movableSymbols(sources) {
  return uniqueSorted(sources.filter((s) => s.current_quantity > 0).map((s) => s.symbol));
}

// Where `symbol` can be moved from: each non-empty source, biggest first.
export function originOptions(sources, symbol) {
  return sourceOptions(sources.filter((s) => s.symbol === symbol && s.current_quantity > 0));
}

// Every BARCA with a target or a holding.
export function barcaNames(report) {
  return uniqueSorted([...(report.barcaAllocations || []), ...(report.barcaActualAllocations || [])].map((b) => b.barca));
}

export function groupsIn(sources, barca) {
  return uniqueSorted(sources.filter((s) => s.barca === barca).map((s) => s.group_name));
}

// Staying in the origin's BARCA keeps the origin's group, so changing only
// the exchange really only changes the exchange. Otherwise: the group
// `symbol` already has in `barca`, else that BARCA's first group, else the
// BARCA's own name (a BARCA with nothing in it yet).
export function defaultGroup(sources, symbol, barca, origin = null) {
  if (origin && origin.barca === barca) return origin.group_name ?? "";
  const own = sources.find((s) => s.symbol === symbol && s.barca === barca && s.group_name);
  return own?.group_name ?? groupsIn(sources, barca)[0] ?? barca;
}

// Every exchange named anywhere in the account, for suggestions.
export function exchangeNames(sources) {
  return uniqueSorted(sources.map((s) => sourceName(s.notes)));
}

// How much the destination source holds now, whether its position
// (symbol + group + BARCA) exists at all, and the group and exchange as
// stored (a typed "base" is the existing "Base").
export function destination(sources, { symbol, assetClass, group, barca, exchange }) {
  const ofAsset = sources.filter((s) => s.symbol === symbol && s.asset_class === assetClass);
  const exact = ofAsset.filter((s) => (s.group_name || "") === group.trim() && (s.barca || "") === barca);
  const position = exact.length ? exact : ofAsset.filter((s) => sameName(s.group_name, group) && sameName(s.barca, barca));
  const source =
    position.find((s) => sourceName(s.notes) === sourceName(exchange)) ??
    position.find((s) => sameName(s.notes, exchange));
  return {
    held: source?.current_quantity ?? 0,
    newPosition: position.length === 0,
    group: position[0]?.group_name ?? group.trim(),
    exchange: source ? sourceName(source.notes) : sourceName(exchange),
  };
}

export function isSamePlace(origin, { group, barca, exchange }) {
  return sameName(origin.group_name, group) && sameName(origin.barca, barca) && sameName(origin.notes, exchange);
}

// The preview and what to fix before sending. `received` defaults to
// `quantity` (no fee).
// `newPosition`: only a new position needs a group (an existing one may
// have been imported without one).
export function previewMove({ held, quantity, received, destHeld, samePlace, group, barca, newPosition = true }) {
  const arrives = received ?? quantity;
  const problems = [];
  if (quantity === null) problems.push("Enter a quantity above 0.");
  else if (quantity > held + 1e-9) problems.push(`The origin holds only ${formatQuantity(held)}.`);
  // Until there's a quantity, that is the only thing missing.
  if (received === null) {
    if (quantity !== null) problems.push("Enter the quantity received, above 0.");
  }
  else if (quantity !== null && received > quantity + 1e-9) problems.push("You can't receive more than you sent.");
  if (!barca) problems.push("Pick the destination BARCA.");
  if (newPosition && !group.trim()) problems.push("Name the destination group.");
  if (samePlace && quantity !== null) problems.push("The destination is the same as the origin.");
  return {
    fromAfter: quantity === null ? null : Math.max(0, held - quantity),
    toAfter: arrives === null ? null : destHeld + arrives,
    fee: quantity !== null && arrives !== null ? Math.max(0, quantity - arrives) : 0,
    problems,
  };
}

// The BARCA shares before and after the move, for the BARCAs it touches
// (empty when it stays in one BARCA with no fee: nothing changes).
export function barcaShift(report, { fromBarca, toBarca, sentValue, receivedValue }) {
  const values = new Map((report.barcaActualAllocations || []).map((b) => [b.barca, b.value || 0]));
  const total = Array.from(values.values()).reduce((sum, v) => sum + v, 0);
  if (total <= 0 || (fromBarca === toBarca && sentValue === receivedValue)) return [];
  const totalAfter = total - (sentValue - receivedValue);
  const share = (v, t) => (t > 0 ? (v / t) * 100 : 0);
  const touched = fromBarca === toBarca ? [fromBarca] : [fromBarca, toBarca];
  return touched.map((barca) => {
    const before = values.get(barca) ?? 0;
    const after = before - (barca === fromBarca ? sentValue : 0) + (barca === toBarca ? receivedValue : 0);
    return { barca, before: share(before, total), after: share(after, totalAfter) };
  });
}

function ref(source) {
  return {
    symbol: source.symbol,
    group_name: source.group_name ?? null,
    barca: source.barca ?? null,
    asset_class: source.asset_class,
    source: sourceName(source.notes) || null,
  };
}

// The POST /transfers body.
export function buildTransferRequest({ origin, group, barca, exchange, quantity, received }) {
  return {
    from: ref(origin),
    to: ref({ ...origin, group_name: group.trim(), barca, notes: exchange }),
    quantity,
    received: received === quantity ? null : received,
  };
}
