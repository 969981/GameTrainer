// SPDX-License-Identifier: GPL-2.0-or-later
// GameTrainer HGSS M8 diagnostic bridge for a source-patched DeSmuME.
// Copyright (C) 2026 GameTrainer contributors. Diagnostic / read-only.
// C++11, Win32, MSVC. Build only when GDB_STUB + GT_HGSS_M8_BRIDGE defined.
//
// The NDSSystem.cpp integration feeds the existing NDS_LoadROM streaming CRC
// byte chunks. Do not read a user-entered file path and call it loaded ROM.
// This bridge does not pause CPUs, patch memory, or validate JIT/Overlay safety.
#pragma once
#if !defined(_WIN32) || !defined(GDB_STUB) || !defined(GT_HGSS_M8_BRIDGE)
#error "Build the HGSS M8 bridge only for Windows DeSmuME with GDB_STUB enabled"
#endif
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <wincrypt.h>
#include <stdint.h>
#include <atomic>
#include <cstring>
#include <cwchar>
#include <mutex>
#include "gt_dp_m10_bridge.h"

#ifdef _MSC_VER
#pragma comment(lib, "advapi32.lib")
#endif

namespace gthgss_m8 {
#pragma pack(push,1)
struct Wire {
    char magic[8];             // "GTHGSS08"
    uint32_t version;          // 1
    uint32_t pid;
    uint64_t created_filetime;
    uint64_t rom_epoch;
    char code[4];              // NDS header 0x0C
    uint8_t revision;          // NDS header 0x1E
    uint64_t rom_size;         // uncompressed reader bytes
    uint32_t crc32;
    uint8_t sha1[20];          // on ROM reader stream, not input filename
    uint8_t loaded;
    uint16_t arm9_port;
    uint16_t arm7_port;
    uint8_t jit_at_load;       // observation, NOT JIT/ICache validation
    uint8_t hash_complete;
};
#pragma pack(pop)
static_assert(sizeof(Wire) == 76, "M8 wire layout must match Rust");

inline uint64_t filetime64(const FILETIME& f) {
    return (uint64_t(f.dwHighDateTime) << 32) | uint64_t(f.dwLowDateTime);
}
struct State {
    std::mutex mu;
    Wire wire;
    HCRYPTPROV provider;
    HCRYPTHASH digest;
    uint64_t hashed_bytes;
    bool hash_ok;
    std::atomic<bool> running;
    HANDLE thread;
    wchar_t pipe_name[96];

    State():provider(0),digest(0),hashed_bytes(0),hash_ok(false),
        running(false),thread(NULL) {
        std::memset(&wire,0,sizeof(wire));
        std::memcpy(wire.magic,"GTHGSS08",8);
        wire.version=1;
        wire.pid=GetCurrentProcessId();
        FILETIME created,exited,kernel,user;
        if(GetProcessTimes(GetCurrentProcess(),&created,&exited,&kernel,&user))
            wire.created_filetime=filetime64(created);
        std::swprintf(pipe_name,sizeof(pipe_name)/sizeof(pipe_name[0]),
            L"\\\\.\\pipe\\GameTrainer.HGSS.M8.%lu",wire.pid);
    }
};
inline State& state() {static State s; return s;}

inline void release_digest(State& s) {
    if(s.digest) {CryptDestroyHash(s.digest);s.digest=0;}
    if(s.provider) {CryptReleaseContext(s.provider,0);s.provider=0;}
    s.hash_ok=false;s.hashed_bytes=0;
}

// Called BEFORE opening the next game: stale ROM identity is immediately revoked.
inline void rom_begin() {
    gtdp_m10::rom_begin(); // revoke both digest protocols on every ROM load attempt
    State& s=state();std::lock_guard<std::mutex> g(s.mu);
    release_digest(s);
    ++s.wire.rom_epoch;
    s.wire.loaded=0;s.wire.hash_complete=0;
    s.wire.rom_size=0;s.wire.crc32=0;
    s.wire.revision=0;s.wire.jit_at_load=0;
    std::memset(s.wire.code,0,4);std::memset(s.wire.sha1,0,20);
}
inline void hash_begin() {
    gtdp_m10::hash_begin();
    State& s=state();std::lock_guard<std::mutex> g(s.mu);
    release_digest(s);
    if(!CryptAcquireContextW(&s.provider,NULL,NULL,PROV_RSA_AES,CRYPT_VERIFYCONTEXT))
        return;
    if(!CryptCreateHash(s.provider,CALG_SHA1,0,0,&s.digest)) {
        release_digest(s);return;
    }
    s.hash_ok=true;
}
// Feed EXACTLY the chunks consumed by NDS_LoadROM's own ROM CRC loop.
inline void hash_chunk(const void* bytes,DWORD length) {
    gtdp_m10::hash_chunk(bytes,length);
    State& s=state();std::lock_guard<std::mutex> g(s.mu);
    if(!s.hash_ok||!s.digest||!bytes||!length) {s.hash_ok=false;return;}
    if(!CryptHashData(s.digest,reinterpret_cast<const BYTE*>(bytes),length,0)) {
        s.hash_ok=false;return;
    }
    s.hashed_bytes+=length;
}
// Called after a successful ROM load and NDS_Reset. A disk-streaming ROM can
// change after hashing; require DeSmuME's in-memory ROM reader as well.
// If any byte is missing,
// no valid ROM record is published (including compressed or trimmed ROMs).
inline void rom_commit(const char code[4],uint8_t revision,uint64_t size,
                       uint32_t crc32,bool jit_enabled,bool resident_rom_reader) {
    gtdp_m10::rom_commit(code,revision,size,crc32,jit_enabled,resident_rom_reader);
    State& s=state();std::lock_guard<std::mutex> g(s.mu);
    BYTE hash[20]={0};DWORD count=20;
    const bool hgss=code && code[0]=='I'&&code[1]=='P' &&
        (code[2]=='K'||code[2]=='G');
    const bool ok=hgss&&resident_rom_reader&&s.hash_ok&&s.digest&&size>=1024*1024
        &&s.hashed_bytes==size&&CryptGetHashParam(s.digest,HP_HASHVAL,hash,&count,0)
        &&count==20;
    if(ok) {
        std::memcpy(s.wire.code,code,4);
        s.wire.revision=revision;s.wire.rom_size=size;
        s.wire.crc32=crc32;std::memcpy(s.wire.sha1,hash,20);
        s.wire.jit_at_load=jit_enabled?1:0;
        s.wire.hash_complete=1;s.wire.loaded=1;
    }
    release_digest(s);
}
inline void rom_unloaded() {rom_begin();}

inline DWORD WINAPI pipe_worker(void*) {
    State& s=state();
    while(s.running.load()) {
        HANDLE h=CreateNamedPipeW(s.pipe_name,PIPE_ACCESS_OUTBOUND,
            PIPE_TYPE_BYTE|PIPE_READMODE_BYTE|PIPE_WAIT,1,
            sizeof(Wire),0,1000,NULL);
        if(h==INVALID_HANDLE_VALUE)break;
        const BOOL connected=ConnectNamedPipe(h,NULL) ||
            GetLastError()==ERROR_PIPE_CONNECTED;
        if(connected&&s.running.load()) {
            Wire copy;
            {std::lock_guard<std::mutex> lock(s.mu);copy=s.wire;}
            DWORD written=0;
            if(WriteFile(h,&copy,sizeof(copy),&written,NULL) &&
               written==sizeof(copy)) {
                // Flush only after client reads; no game-memory operation here.
                FlushFileBuffers(h);
            }
        }
        DisconnectNamedPipe(h);
        CloseHandle(h);
    }
    return 0;
}

// Start in Windows main.cpp AFTER both GDB stubs have been created.
// The ports are only declared here; GameTrainer independently checks Windows
// TCP owner PID rather than trusting these claimed port numbers.
inline bool start(uint16_t arm9_port,uint16_t arm7_port) {
    if(arm9_port<1024||arm7_port<1024||arm9_port==arm7_port)return false;
    State& s=state();
    if(s.running.load())return false;
    {
        std::lock_guard<std::mutex> g(s.mu);
        s.wire.arm9_port=arm9_port;
        s.wire.arm7_port=arm7_port;
    }
    s.running.store(true);
    s.thread=CreateThread(NULL,0,pipe_worker,NULL,0,NULL);
    if(!s.thread){s.running.store(false);return false;}
    // HGSS M8 keeps its original 76-byte pipe, DP M10 has a separate 89-byte pipe.
    return gtdp_m10::start(arm9_port,arm7_port);
}
inline void stop() {
    gtdp_m10::stop();
    State& s=state();
    if(!s.running.exchange(false))return;
    // Wake the server if blocked in ConnectNamedPipe.
    HANDLE wake=CreateFileW(s.pipe_name,GENERIC_READ,0,NULL,OPEN_EXISTING,0,NULL);
    if(wake!=INVALID_HANDLE_VALUE)CloseHandle(wake);
    if(s.thread) {
        WaitForSingleObject(s.thread,3000);
        CloseHandle(s.thread);s.thread=NULL;
    }
    std::lock_guard<std::mutex> g(s.mu);
    release_digest(s);
    s.wire.loaded=0;
    s.wire.hash_complete=0;
}
} // namespace gthgss_m8
