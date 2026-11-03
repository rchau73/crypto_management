import { describe, expect, it } from "vitest";
import { planRound } from "./rebalance";

const TARGETS = { Base: 50, Altcoins: 10, RendaPassiva: 10, Caixa: 30 };

// Builds a report from [symbol, barca, value, assetTarget] rows.
function report(positions, targets = TARGETS) {
  return {
    allocations: positions.map(([symbol, barca, value, target]) => ({
      symbol,
      group: "G",
      barca,
      value,
      target_percent: target,
      price: 1,
      current_quantity: value,
    })),
    barcaAllocations: Object.entries(targets).map(([barca, target_percent]) => ({ barca, target_percent, value: 0 })),
    barcaActualAllocations: Object.keys(targets).map((barca) => ({
      barca,
      value: positions.filter((p) => p[1] === barca).reduce((s, p) => s + p[2], 0),
    })),
  };
}

// BARCA.md, worked example A: a crash ($100,000 wallet).
const crash = report([
  ["BTC", "Base", 38000, 50],
  ["USDT", "Caixa", 40000, 30],
  ["ETH", "Altcoins", 4500, 5],
  ["SOL", "Altcoins", 2500, 3],
  ["LINK", "Altcoins", 2000, 2],
  ["FII-A", "RendaPassiva", 7000, 5],
  ["FII-B", "RendaPassiva", 6000, 5],
]);

// BARCA.md, worked example B: a rally.
const rally = report([
  ["BTC", "Base", 64000, 50],
  ["USDT", "Caixa", 20000, 30],
  ["ETH", "Altcoins", 6000, 5],
  ["SOL", "Altcoins", 3000, 3],
  ["LINK", "Altcoins", 2000, 2],
  ["FII-A", "RendaPassiva", 3000, 5],
  ["FII-B", "RendaPassiva", 2000, 5],
]);

const brief = (steps) => steps.map((s) => `${s.verb} ${s.symbol} ${Math.round(s.amount)}`);

describe("planRound", () => {
  it("example A, extreme fear: buy $3,600 of BTC from Caixa, never sell", () => {
    const plan = planRound(crash, "fear");
    expect(brief(plan.steps)).toEqual(["Buy BTC 3600"]);
    expect(plan.steps[0].funding).toBe("Funded from Caixa");
    expect(plan.waiting).toEqual([{ barca: "RendaPassiva", reason: "above its band; sell when the fear passes, never into it." }]);
  });

  it("example B, extreme greed: sell $4,200 of BTC into Caixa, never buy", () => {
    const plan = planRound(rally, "greed");
    expect(brief(plan.steps)).toEqual(["Sell BTC 4200"]);
    expect(plan.steps[0].funding).toBe("Proceeds to Caixa");
    expect(plan.waiting.map((w) => w.barca)).toEqual(["RendaPassiva"]);
    expect(plan.notes).toEqual(["Caixa is below its band: sells refill it."]);
  });

  it("neutral works both ways, Base first, splitting a BARCA's step by its assets' gaps", () => {
    // RendaPassiva is $3,000 over: step $900, FII-A is $2,000 over, FII-B $1,000.
    expect(brief(planRound(crash, "neutral").steps)).toEqual(["Buy BTC 3600", "Sell FII-A 600", "Sell FII-B 300"]);
  });

  it("fear holds when Caixa is not above its target", () => {
    const plan = planRound(rally, "fear");
    expect(plan.steps).toEqual([]);
    expect(plan.waiting).toEqual([
      { barca: "Base", reason: "above its band; sell when the fear passes, never into it." },
      { barca: "RendaPassiva", reason: "below its band, but Caixa is not above its target, so hold." },
    ]);
  });

  it("fear never spends Caixa below its target", () => {
    // Caixa $31,000 of $100,000: only $1,000 over its 30% target.
    const lean = report([
      ["BTC", "Base", 38000, 50],
      ["USDT", "Caixa", 31000, 30],
      ["ETH", "Altcoins", 10000, 10],
      ["FII", "RendaPassiva", 21000, 10],
    ]);
    expect(brief(planRound(lean, "fear").steps)).toEqual(["Buy BTC 1000"]);
  });

  it("lists nothing to do inside the bands, and no steps before a sentiment is picked", () => {
    const calm = report([
      ["BTC", "Base", 52000, 50],
      ["USDT", "Caixa", 29000, 30],
      ["ETH", "Altcoins", 9500, 10],
      ["FII", "RendaPassiva", 9500, 10],
    ]);
    expect(planRound(calm, "neutral")).toMatchObject({ steps: [], waiting: [], outOfBandCount: 0 });
    const unpicked = planRound(crash, null);
    expect(unpicked.steps).toEqual([]);
    expect(unpicked.outOfBandCount).toBe(3); // Base, RendaPassiva, Caixa
    expect(unpicked.barcas.map((b) => b.name)).toEqual(["Base", "Caixa", "Altcoins", "RendaPassiva"]); // by target
  });

  it("sums a symbol held in several groups before splitting the step", () => {
    const split = report([
      ["BTC", "Base", 19000, 25],
      ["BTC", "Base", 19000, 25],
      ["USDT", "Caixa", 40000, 30],
      ["ETH", "Altcoins", 9000, 10],
      ["FII", "RendaPassiva", 13000, 10],
    ]);
    expect(brief(planRound(split, "fear").steps)).toEqual(["Buy BTC 3600"]);
  });

  it("explains an out-of-band BARCA whose assets are all on target the other way", () => {
    // Altcoins' only asset has no target, so the BARCA is short but no asset is.
    const odd = report([
      ["BTC", "Base", 50000, 50],
      ["USDT", "Caixa", 45000, 30],
      ["MEME", "Altcoins", 1000, 0],
      ["FII", "RendaPassiva", 4000, 20],
    ]);
    const plan = planRound(odd, "neutral");
    expect(plan.waiting).toContainEqual({
      barca: "Altcoins",
      reason: "out of band, but none of its assets is off target on that side (check its asset targets).",
    });
  });

  it("is empty for an account with no holdings", () => {
    expect(planRound({ allocations: [] }, "fear")).toMatchObject({ steps: [], waiting: [], outOfBandCount: 0, totalValue: 0 });
  });
});
