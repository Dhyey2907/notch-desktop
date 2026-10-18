import { describe, expect, it } from "vitest";

import { caller, callKind } from "./calls";

describe("phone calls", () => {
  it("spots incoming and missed calls", () => {
    expect(callKind("Rahul", "Incoming call")).toBe("incoming");
    expect(callKind("Incoming voice call", "Mom")).toBe("incoming");
    expect(callKind("Rahul", "Missed call")).toBe("missed");
    expect(callKind("+91 98765 43210", "Missed video call")).toBe("missed");
    expect(callKind("Rahul", "Call me when free")).toBeNull();
  });

  it("finds who's calling", () => {
    expect(caller("Rahul", "Incoming call")).toBe("Rahul");
    expect(caller("Incoming call", "Mom")).toBe("Mom");
    expect(caller("Missed call", "")).toBe("Unknown caller");
  });
});
