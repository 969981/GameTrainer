// SPDX-License-Identifier: GPL-2.0-or-later
// M10: Diamond/Pearl ROM identity sub-bridge for the M8 DeSmuME source build.
// In-core 4096-byte ROM Reader stream SHA-256; Windows named pipe read-only.
// This does not pause ARM9/ARM7, inspect/flush JIT or patch Guest memory.
#pragma once
#if !defined(_WIN32) || !defined(GDB_STUB) || !defined(GT_HGSS_M8_BRIDGE)
#error "M10 requires the optional Windows GDB_STUB + GT_HGSS_M8_BRIDGE build"
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

#ifdef _MSC_VER
#pragma comment(lib, "advapi32.lib")
#endif

namespace gtdp_m10 {
#pragma pack(push, 1)
struct Wire {
    char magic[8];          // GTPDPM10
    uint32_t version;       // 1
    uint32_t pid;
    uint64_t created_filetime;
    uint64_t rom_epoch;
    char code[4];           // NDS header bytes 0x0C..0x10
    uint8_t revision;       // NDS header byte 0x1E
    uint8_t header6c;       // NDS ROM reader byte 0x6C
    uint64_t rom_size;      // complete in-memory ROM Reader byte count
    uint32_t crc32;
    uint8_t sha256[32];     // raw bytes streamed to NDS_LoadROM CRC loop
    uint8_t loaded;
    uint16_t arm9_port;
    uint16_t arm7_port;
    uint8_t jit_at_load;    // observed setting only; not ICache coherence
    uint8_t hash_complete;
};
#pragma pack(pop)
static_assert(sizeof(Wire) == 89, "M10 wire must equal trainer-dp/m10.rs");

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
    bool captured_header;
    uint8_t first_header6c;
    std::atomic<bool> running;
    HANDLE thread;
    wchar_t pipe_name[96];

    State():provider(0),digest(0),hashed_bytes(0),hash_ok(false),
        captured_header(false),first_header6c(0),running(false),thread(NULL) {
        std::memset(&wire,0,sizeof(wire));
        std::memcpy(wire.magic,"GTPDPM10",8);
        wire.version=1;
        wire.pid=GetCurrentProcessId();
        FILETIME created,exited,kernel,user;
        if(GetProcessTimes(GetCurrentProcess(),&created,&exited,&kernel,&user))
            wire.created_filetime=filetime64(created);
        std::swprintf(pipe_name,sizeof(pipe_name)/sizeof(pipe_name[0]),
            L"\\\\.\\pipe\\GameTrainer.DP.M10.%lu",wire.pid);
    }
};
inline State& state() {static State s;return s;}
inline void reset_digest(State& s) {
    if(s.digest){CryptDestroyHash(s.digest);s.digest=0;}
    if(s.provider){CryptReleaseContext(s.provider,0);s.provider=0;}
    s.hashed_bytes=0;s.hash_ok=false;s.captured_header=false;s.first_header6c=0;
}
inline void rom_begin() {
    State& s=state();std::lock_guard<std::mutex> lock(s.mu);
    reset_digest(s);
    ++s.wire.rom_epoch;
    s.wire.loaded=0;s.wire.hash_complete=0;
    s.wire.rom_size=0;s.wire.crc32=0;s.wire.header6c=0;
    s.wire.revision=0;s.wire.jit_at_load=0;
    std::memset(s.wire.code,0,4);std::memset(s.wire.sha256,0,32);
}
inline void hash_begin() {
    State& s=state();std::lock_guard<std::mutex> lock(s.mu);
    reset_digest(s);
    if(!CryptAcquireContextW(&s.provider,NULL,NULL,PROV_RSA_AES,CRYPT_VERIFYCONTEXT))
        return;
    if(!CryptCreateHash(s.provider,CALG_SHA_256,0,0,&s.digest)) {
        reset_digest(s);return;
    }
    s.hash_ok=true;
}
inline void hash_chunk(const void* chunk,DWORD length) {
    State& s=state();std::lock_guard<std::mutex> lock(s.mu);
    if(!s.hash_ok||!s.digest||!chunk||!length||length>4096) {
        s.hash_ok=false;return;
    }
    const BYTE* bytes=reinterpret_cast<const BYTE*>(chunk);
    if(s.hashed_bytes==0) {
        // The first chunk must include NDS header offset 0x6C. Never infer
        // Header6C from a user-provided ROM filename or other Guest memory.
        if(length<=0x6C) {s.hash_ok=false;return;}
        s.first_header6c=bytes[0x6C];
        s.captured_header=true;
    }
    if(!CryptHashData(s.digest,bytes,length,0)) {s.hash_ok=false;return;}
    s.hashed_bytes+=length;
}
inline void rom_commit(const char code[4],uint8_t revision,uint64_t rom_size,
                       uint32_t crc32,bool jit_enabled,bool resident_rom_reader) {
    State& s=state();std::lock_guard<std::mutex> lock(s.mu);
    BYTE hash[32]={0};DWORD count=32;
    const bool is_dp=code&&code[0]=='A'&&
        (code[1]=='D'||code[1]=='P')&&code[2]=='A'&&
        (code[3]=='E'||code[3]=='F'||code[3]=='D'||
         code[3]=='I'||code[3]=='S');
    const bool good=is_dp&&resident_rom_reader&&s.hash_ok&&s.captured_header
        &&s.digest&&rom_size>=1024*1024&&rom_size<=512ull*1024*1024
        &&s.hashed_bytes==rom_size
        &&CryptGetHashParam(s.digest,HP_HASHVAL,hash,&count,0)&&count==32;
    if(good) {
        std::memcpy(s.wire.code,code,4);
        s.wire.revision=revision;s.wire.header6c=s.first_header6c;
        s.wire.rom_size=rom_size;s.wire.crc32=crc32;
        std::memcpy(s.wire.sha256,hash,32);
        s.wire.jit_at_load=jit_enabled?1:0;
        s.wire.hash_complete=1;s.wire.loaded=1;
    }
    reset_digest(s);
}
inline DWORD WINAPI pipe_worker(void*) {
    State& s=state();
    while(s.running.load()) {
        HANDLE pipe=CreateNamedPipeW(s.pipe_name,PIPE_ACCESS_OUTBOUND,
            PIPE_TYPE_BYTE|PIPE_READMODE_BYTE|PIPE_WAIT,1,sizeof(Wire),0,1000,NULL);
        if(pipe==INVALID_HANDLE_VALUE)break;
        const BOOL connected=ConnectNamedPipe(pipe,NULL)||
            GetLastError()==ERROR_PIPE_CONNECTED;
        if(connected&&s.running.load()){
            Wire copy;
            {std::lock_guard<std::mutex> lock(s.mu);copy=s.wire;}
            DWORD written=0;
            if(WriteFile(pipe,&copy,sizeof(copy),&written,NULL)&&
               written==sizeof(copy))FlushFileBuffers(pipe);
        }
        DisconnectNamedPipe(pipe);
        CloseHandle(pipe);
    }
    return 0;
}
inline bool start(uint16_t arm9_port,uint16_t arm7_port){
    if(arm9_port<1024||arm7_port<1024||arm9_port==arm7_port)return false;
    State& s=state();
    if(s.running.load())return false;
    {
        std::lock_guard<std::mutex> lock(s.mu);
        s.wire.arm9_port=arm9_port;s.wire.arm7_port=arm7_port;
    }
    s.running.store(true);
    s.thread=CreateThread(NULL,0,pipe_worker,NULL,0,NULL);
    if(!s.thread){s.running.store(false);return false;}
    return true;
}
inline void stop(){
    State& s=state();
    if(!s.running.exchange(false))return;
    HANDLE wake=CreateFileW(s.pipe_name,GENERIC_READ,0,NULL,
        OPEN_EXISTING,0,NULL);
    if(wake!=INVALID_HANDLE_VALUE)CloseHandle(wake);
    if(s.thread){
        WaitForSingleObject(s.thread,3000);
        CloseHandle(s.thread);s.thread=NULL;
    }
    std::lock_guard<std::mutex> lock(s.mu);
    reset_digest(s);s.wire.loaded=0;s.wire.hash_complete=0;
}
} // namespace gtdp_m10
