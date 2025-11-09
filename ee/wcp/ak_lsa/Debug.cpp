#include "include/Debug.h"
#include <Windows.h>
#include <string>
#include <mutex>
#include <fstream>
#include <iostream>

using std::ofstream;
std::mutex g_dbgMutex;

void LOG(const char* data) {
    std::fstream fs("c:\\ak_lsa.txt", std::ios_base::app);

    if(fs) {
        fs << data << std::endl;
        fs.flush();
        fs.close();
    }
}
