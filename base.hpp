#pragma once

#define WIN32_LEAN_AND_MEAN
#include <Windows.h>

#include <cstdio>
#include <iostream>

inline void con_allocate(bool no_flush) noexcept {
  AllocConsole();
  FILE* out;
  freopen_s(&out, "CON", "w", stdout);
  if (no_flush) {
    std::ios_base::sync_with_stdio(false);
    std::setvbuf(stdout, nullptr, _IOFBF, BUFSIZ);
  }
}

inline void con_noflush() noexcept {}

void reloader_main();

BOOL DllMain(HINSTANCE hinstDll, DWORD fdwReason, LPVOID lpvReserved) {
  if (fdwReason == DLL_PROCESS_ATTACH)
    CreateThread(NULL, 0, (LPTHREAD_START_ROUTINE)&reloader_main, NULL, 0,
      NULL);
  return TRUE;
}
