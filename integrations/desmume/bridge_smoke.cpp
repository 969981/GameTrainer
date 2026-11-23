// SPDX-License-Identifier: GPL-2.0-or-later
// Standalone MSVC smoke test for the optional DeSmuME M8 bridge header.
// This uses synthetic bytes, never a ROM image, emulator or Guest memory.
#define GDB_STUB 1
#define GT_HGSS_M8_BRIDGE 1
#include "gt_hgss_m8_bridge.h"
#include <vector>
#include <cstdio>

int main() {
    using namespace gthgss_m8;
    if(sizeof(Wire)!=76) return 10;
    std::vector<uint8_t> bytes(1024*1024,0x42);
    rom_begin();
    hash_begin();
    hash_chunk(bytes.data(),static_cast<DWORD>(bytes.size()));
    rom_commit("IPKJ",0,bytes.size(),0x12345678,false,true);
    {
        std::lock_guard<std::mutex> g(state().mu);
        if(!state().wire.loaded || !state().wire.hash_complete ||
           state().wire.rom_size!=bytes.size() ||
           state().wire.rom_epoch!=1) return 11;
    }
    rom_begin();
    hash_begin();
    hash_chunk(bytes.data(),static_cast<DWORD>(bytes.size()));
    // A disk-backed stream is not a stable in-memory ROM snapshot.
    rom_commit("IPKJ",0,bytes.size(),0x12345678,false,false);
    {
        std::lock_guard<std::mutex> g(state().mu);
        if(state().wire.loaded || state().wire.hash_complete) return 12;
    }
    rom_unloaded();
    {
        std::lock_guard<std::mutex> g(state().mu);
        if(state().wire.rom_epoch!=3 || state().wire.loaded) return 13;
    }
    if(sizeof(gtdp_m10::Wire)!=89) return 20;
    rom_begin();
    hash_begin();
    for(size_t i=0;i<bytes.size();i+=4096)
        hash_chunk(bytes.data()+i,4096);
    rom_commit("APAE",13,bytes.size(),0xAABBCCDD,false,true);
    {
        std::lock_guard<std::mutex> g(gtdp_m10::state().mu);
        const gtdp_m10::Wire& p=gtdp_m10::state().wire;
        if(!p.loaded||!p.hash_complete||p.rom_epoch!=4||
           std::memcmp(p.code,"APAE",4)!=0||p.revision!=13||
           p.header6c!=0x42||p.rom_size!=bytes.size())return 21;
        const char expected[]="5ae9782017a68037004b2bf806c77d324db4d915ed3725d84eb3121b2ad16061";
        char observed[65]={0};
        for(int i=0;i<32;i++)
            std::sprintf(observed+i*2,"%02x",static_cast<unsigned int>(p.sha256[i]));
        if(std::strcmp(expected,observed)!=0)return 22;
    }
    // The M8/HGSS wire remains 76 bytes and does not claim DP is HGSS.
    {
        std::lock_guard<std::mutex> g(state().mu);
        if(state().wire.loaded||state().wire.hash_complete)return 23;
    }
    rom_begin();
    hash_begin();
    for(size_t i=0;i<bytes.size();i+=4096)
        hash_chunk(bytes.data()+i,4096);
    rom_commit("APAE",13,bytes.size(),0xAABBCCDD,false,false);
    {
        std::lock_guard<std::mutex> g(gtdp_m10::state().mu);
        if(gtdp_m10::state().wire.loaded||
           gtdp_m10::state().wire.hash_complete)return 24;
    }
    rom_unloaded();
    {
        std::lock_guard<std::mutex> g(gtdp_m10::state().mu);
        if(gtdp_m10::state().wire.loaded||
           gtdp_m10::state().wire.hash_complete)return 25;
    }
    std::puts("M8 SHA1 + DP M10 SHA256, memory-only, unload smoke passed");
    return 0;
}
