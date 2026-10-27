import { describe, expect, it } from "vitest";
import { describeImportSummary } from "./importSummary";

describe("describeImportSummary", () => {
  it("lists the counts", () => {
    const text = describeImportSummary({
      positions_added: 1,
      sources_added: 2,
      sources_updated: 3,
      sources_unchanged: 4,
      missing_from_csv: [],
      target_conflicts: [],
    });
    expect(text).toBe(
      "Imported CSV: 1 new position(s); 2 source(s) added; 3 updated; 4 unchanged",
    );
  });

  it("calls out sources missing from the CSV and ignored targets", () => {
    const text = describeImportSummary({
      missing_from_csv: [
        { symbol: "LINK", group_name: "DeFi", notes: "Coinbase", current_quantity: 22.8 },
        { symbol: "ETH", group_name: "Core", notes: null, current_quantity: 1 },
      ],
      target_conflicts: [{ symbol: "BTC", database_percent: 40, csv_percent: 99 }],
    });
    expect(text).toContain("not in CSV, left as is: LINK (DeFi / Coinbase), ETH (Core)");
    expect(text).toContain("CSV target ignored (database kept) for: BTC");
  });

  it("survives a missing or partial response", () => {
    expect(describeImportSummary()).toBe(
      "Imported CSV: 0 new position(s); 0 source(s) added; 0 updated; 0 unchanged",
    );
  });
});
