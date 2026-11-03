import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatusLine } from "./StatusLine";

describe("StatusLine", () => {
  it("renders nothing when there's no update, status, or error", () => {
    const { container } = render(<StatusLine pricesAsOf={null} importStatus="" importError="" />);
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the import status text on success", () => {
    render(<StatusLine pricesAsOf={null} importStatus="Imported 3 wallet rows from CSV" importError="" />);
    expect(screen.getByText("Imported 3 wallet rows from CSV")).toBeInTheDocument();
  });

  it("shows the import error inline, in its own paragraph separate from the status line", () => {
    render(<StatusLine pricesAsOf={null} importStatus="" importError="Invalid CSV file: missing field `symbol`" />);
    const error = screen.getByText("Invalid CSV file: missing field `symbol`");
    expect(error.tagName).toBe("P");
  });

  it("shows the prices' time in BRT and the error together when a later import fails", () => {
    render(
      <StatusLine
        pricesAsOf="2026-01-01T12:00:00+00:00"
        importStatus=""
        importError="Invalid CSV file: missing field `symbol`"
      />
    );
    expect(screen.getByText(/Prices as of 2026-01-01 09:00:00 \(BRT\)/)).toBeInTheDocument();
    expect(screen.getByText("Invalid CSV file: missing field `symbol`")).toBeInTheDocument();
  });
});
