import {describe,expect,it} from "vitest";
import {M11_ACK,hexBytes,m11SourceAllowed,m11Valid} from "./dp-m11";
describe("DP M11 guarded experimental patch client",()=>{
  it("permits only source audited complete-ROM profiles and patch variants",()=>{
    expect(m11SourceAllowed("APAE",13,"wall_collision")).toBe(true);
    expect(m11SourceAllowed("APAE",13,"capture_english")).toBe(true);
    expect(m11SourceAllowed("APAI",5,"wall_collision")).toBe(true);
    expect(m11SourceAllowed("APAI",5,"capture_english")).toBe(false);
    expect(m11SourceAllowed("ADAE",13,"wall_collision")).toBe(false);
    expect(m11SourceAllowed("APAJ",6,"capture_english")).toBe(false);
  });
  it("requires source ROM, PID, distinct ARM9 and ARM7 GDB ports",()=>{
    expect(m11Valid(155,3333,3334,"APAE",13,"wall_collision")).toBe(true);
    expect(m11Valid(0,3333,3334,"APAE",13,"wall_collision")).toBe(false);
    expect(m11Valid(155,3333,3333,"APAE",13,"wall_collision")).toBe(false);
    expect(m11Valid(155,3333,3334,"APAI",5,"capture_english")).toBe(false);
  });
  it("uses explicit opt-in and source halfword changes, not user-defined raw bytes",()=>{
    expect(M11_ACK).toBe("I_ACCEPT_DP_M11_EXPERIMENTAL_GUEST_WRITES");
    expect(hexBytes([0x20,0x1c])).toBe("20 1C");
    expect(hexBytes([0x00,0x20])).toBe("00 20");
    expect(hexBytes([0x01,0x28])).toBe("01 28");
    expect(hexBytes([0x80,0x42])).toBe("80 42");
  });
});
