import { describe, expect, it } from "vitest";

import { findCode, masked } from "./codes";

describe("phone sign-in codes", () => {
  it("finds codes next to code words", () => {
    expect(findCode("Messages", "Your OTP is 482913. Do not share it.")).toBe("482913");
    expect(findCode("Google", "G-123456 is your Google verification code.")).toBe("123456");
    expect(findCode("Bank", "Use 7731 to sign in")).toBe("7731");
  });

  it("leaves ordinary messages alone", () => {
    expect(findCode("Mom", "Call me at 9876543210")).toBeNull();
    expect(findCode("Friend", "See you at 10:30, table 12")).toBeNull();
    expect(findCode("Shop", "Your code expires at 10:30 on 12/10")).toBeNull();
    expect(findCode("Shop", "Order paid: ₹4599 with code")).toBeNull();
  });

  it("masks all but the last two digits", () => {
    expect(masked("482913")).toBe("••••13");
  });
});
