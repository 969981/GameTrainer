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
    std::puts("M8 C++ bridge wire / SHA1 / unload smoke passed");
    return 0;
}
