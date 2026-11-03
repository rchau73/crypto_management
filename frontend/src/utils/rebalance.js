// "What do I do this round?" — BARCA.md's three rules applied to the live
// allocations. Pure, so every rule is unit-tested; the Rebalance page only
// renders what comes out.
//
// Rule 1: only a BARCA outside its ±20% band needs action.
// Rule 2: the market sentiment decides the direction —
//   fear: buy only, and only while Caixa is above its target (Base first);
//   neutral: rebalance both ways, slowly;
//   greed: sell only, into Caixa.
// Rule 3: one step is 30% of the $ gap (DCA), split across the BARCA's
//   assets that are off target on the same side, in proportion to their gaps.
import { buildHoldingRows, DCA_FRACTION } from "./holdings";

// The BARCA names BARCA.md uses for the cash bucket and the core position.
export const CASH_BARCA = "Caixa";
export const BASE_BARCA = "Base";

export const SENTIMENTS = [
  { key: "fear", label: "Fear / Extreme fear" },
  { key: "neutral", label: "Neutral" },
  { key: "greed", label: "Greed / Extreme greed" },
];

export const RULE_TEXT = {
  fear: "Fear: buy only. While Caixa is above its target, move Caixa into the BARCA buckets below their band, Base first. Never sell into fear.",
  neutral: "Neutral: rebalance slowly in both directions, one DCA step per round.",
  greed: "Greed: sell only. Sell the BARCA buckets above their band into Caixa. Never buy into greed.",
};

// Same-side assets of one BARCA, aggregated by symbol (a symbol can sit in
// several groups), with their $ gap to target.
function membersOf(barca, positionRows, side) {
  const bySymbol = new Map();
  positionRows
    .filter((p) => p.barca === barca)
    .forEach((p) => bySymbol.set(p.name, (bySymbol.get(p.name) ?? 0) + p.gap));
  return Array.from(bySymbol, ([symbol, gap]) => ({ symbol, gap })).filter((m) => (side === "Buy" ? m.gap < 0 : m.gap > 0));
}

// Why an out-of-band BARCA gets no step under this sentiment, or null.
function waitReason(row, sentiment, cash) {
  const buying = row.status === "below";
  if (buying && sentiment === "greed") return "below its band; no buying while the market is greedy.";
  if (!buying && sentiment === "fear") return "above its band; sell when the fear passes, never into it.";
  if (buying && sentiment === "fear") {
    if (!cash) return `below its band, but there is no ${CASH_BARCA} bucket to buy with.`;
    if (cash.gap <= 0) return `below its band, but ${CASH_BARCA} is not above its target, so hold.`;
  }
  return null;
}

// `report`: the allocations report (see buildHoldingRows); `sentiment`:
// "fear" | "neutral" | "greed", or null before the user picks one (then
// only the bands are worked out, no steps).
export function planRound(report, sentiment) {
  const { rows, totalValue } = buildHoldingRows("barca", report);
  // Biggest target first (Base leads), untargeted buckets last.
  const barcas = [...rows].sort((a, b) => b.targetPercent - a.targetPercent || a.name.localeCompare(b.name));
  const { rows: positions } = buildHoldingRows("position", report);
  const cash = barcas.find((b) => b.name === CASH_BARCA) ?? null;
  const outOfBand = barcas.filter((b) => b.status !== "in");

  const result = { barcas, totalValue, outOfBandCount: outOfBand.length, steps: [], waiting: [], notes: [] };
  if (cash && cash.status !== "in") {
    result.notes.push(
      cash.status === "above"
        ? `${CASH_BARCA} is above its band: it is the dry powder for buying when fear comes.`
        : `${CASH_BARCA} is below its band: sells refill it.`
    );
  }
  if (!sentiment) return result;

  // Fear spends only Caixa's surplus over its target, never Caixa itself.
  let fearBudget = sentiment === "fear" && cash ? Math.max(0, cash.gap) : Infinity;

  const actionable = outOfBand
    .filter((b) => b.name !== CASH_BARCA)
    .sort((a, b) => {
      // Base first (BARCA.md), then the biggest gap.
      if ((a.name === BASE_BARCA) !== (b.name === BASE_BARCA)) return a.name === BASE_BARCA ? -1 : 1;
      return Math.abs(b.gap) - Math.abs(a.gap);
    });

  actionable.forEach((row) => {
    const reason = waitReason(row, sentiment, cash);
    if (reason) {
      result.waiting.push({ barca: row.name, reason });
      return;
    }
    const verb = row.status === "below" ? "Buy" : "Sell";
    let stepTotal = Math.abs(row.gap) * DCA_FRACTION;
    if (verb === "Buy") {
      stepTotal = Math.min(stepTotal, fearBudget);
      fearBudget -= stepTotal;
      if (stepTotal < 0.01) {
        result.waiting.push({ barca: row.name, reason: `below its band, but ${CASH_BARCA}'s surplus is already used this round.` });
        return;
      }
    }
    const members = membersOf(row.name, positions, verb);
    const gapSum = members.reduce((sum, m) => sum + Math.abs(m.gap), 0);
    if (gapSum <= 0) {
      result.waiting.push({ barca: row.name, reason: "out of band, but none of its assets is off target on that side (check its asset targets)." });
      return;
    }
    members
      .sort((a, b) => Math.abs(b.gap) - Math.abs(a.gap))
      .forEach((m) => {
        const amount = (stepTotal * Math.abs(m.gap)) / gapSum;
        if (amount < 0.01) return;
        result.steps.push({
          id: `${verb}|${row.name}|${m.symbol}`,
          verb,
          symbol: m.symbol,
          barca: row.name,
          amount,
          barcaGap: row.gap,
          funding: verb === "Buy" ? `Funded from ${CASH_BARCA}` : `Proceeds to ${CASH_BARCA}`,
        });
      });
  });
  return result;
}
