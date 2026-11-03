import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DeviationBadge } from "./DeviationBadge";
import { BAND_COLORS } from "../theme";

const rgb = (hex) => `rgb(${[1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(", ")})`;

describe("DeviationBadge", () => {
  it("shows above band in green and below band in red, with the words", () => {
    render(
      <>
        <DeviationBadge status="above">Above band</DeviationBadge>
        <DeviationBadge status="below">Below band</DeviationBadge>
      </>
    );
    expect(screen.getByText("Above band")).toHaveStyle({ color: rgb(BAND_COLORS.above) });
    expect(screen.getByText("Below band")).toHaveStyle({ color: rgb(BAND_COLORS.below) });
  });

  it("leaves an in-band reading uncolored and without a dot", () => {
    const { container } = render(<DeviationBadge status="in">In band</DeviationBadge>);
    expect(container.querySelectorAll("span")).toHaveLength(2); // wrapper + text, no dot
  });
});
