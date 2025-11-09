#include "include/Debug.h"
#include <Windows.h>
#include <string>
#include <mutex>
#include <fstream>
#include <iostream>

using std::ofstream;
std::mutex g_dbgMutex;

void Debug(const char* data) {
    g_dbgMutex.lock();
    std::ofstream fs("c:\\ak_lsa.txt");

    if(fs) {
        fs << data << std::endl;
        fs.flush();
        fs.close();
    }
    g_dbgMutex.unlock();
}
