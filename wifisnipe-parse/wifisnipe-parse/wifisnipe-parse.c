#include <stdio.h>
#include <stdint.h>
#include <stdbool.h>
#include <windows.h>
#include <string.h>

#define NODEMCU_BAUD_RATE CBR_115200

HANDLE hSerial;

void print_error(const char* context)
{
    DWORD error_code = GetLastError();
    char buffer[256];
    DWORD size = FormatMessageA(
        FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_MAX_WIDTH_MASK,
        NULL, error_code, MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US),
        buffer, sizeof(buffer), NULL);
    if (size == 0) { buffer[0] = 0; }
    fprintf(stderr, "%s: %s\n", context, buffer);
}

HANDLE openSerialPort(uint32_t baud_rate) {
    LPCWSTR ldevice = L"\\\\.\\COM3";
    const char* device = "\\\\.\\COM3";
    hSerial = CreateFile(ldevice,
        GENERIC_READ | GENERIC_WRITE,
        0,
        NULL,
        OPEN_EXISTING,
        0,
        NULL);
    if (hSerial == INVALID_HANDLE_VALUE) {
        if (GetLastError() == ERROR_FILE_NOT_FOUND) {
            print_error(device);
            return INVALID_HANDLE_VALUE;
        }
        print_error(device);
        return INVALID_HANDLE_VALUE;
    }

    if (!FlushFileBuffers(hSerial))
    {
        print_error("Failed to flush serial port");
        CloseHandle(hSerial);
        return INVALID_HANDLE_VALUE;
    }

    DCB dcbSerialParams = { 0 };
    dcbSerialParams.DCBlength = sizeof(dcbSerialParams);

    if (!GetCommState(hSerial, &dcbSerialParams)) {
        //error getting state
    }
    dcbSerialParams.BaudRate = baud_rate;
    dcbSerialParams.ByteSize = 8;
    dcbSerialParams.StopBits = ONESTOPBIT;
    dcbSerialParams.Parity = NOPARITY;
    if (!SetCommState(hSerial, &dcbSerialParams)) {
        print_error("Failed to set serial parameters");
        CloseHandle(hSerial);
        return INVALID_HANDLE_VALUE;
    }

    // Timeouts

    COMMTIMEOUTS timeouts = { 0 };
    timeouts.ReadIntervalTimeout = 50;
    timeouts.ReadTotalTimeoutConstant = 100;
    timeouts.ReadTotalTimeoutMultiplier = 0;
    timeouts.WriteTotalTimeoutConstant = 100;
    timeouts.WriteTotalTimeoutMultiplier = 0;
    if (!SetCommTimeouts(hSerial, &timeouts)) {
        print_error("Failed to set serial timeouts");
        CloseHandle(hSerial);
        return INVALID_HANDLE_VALUE;
    }
    return 0;
}


SSIZE_T read_port(HANDLE hSerial, uint8_t* buffer, size_t size)
{
    DWORD received;
    BOOL success = ReadFile(hSerial, buffer, size, &received, NULL);
    if (!success)
    {
        print_error("Failed to read from port");
        CloseHandle(hSerial);
        return -1;
    }
    return received;
}

BOOL WINAPI ExitHandler(DWORD eventCode) {
    /*
        Gere les signaux de fermeture du programme
        Pour permettre un changement de
    */

    switch (eventCode) {
    case CTRL_CLOSE_EVENT | CTRL_BREAK_EVENT | CTRL_C_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT:
        CloseHandle(hSerial);
        ExitProcess(0);
    default:
        return FALSE;
    }
}

void slice(const char* str, char* result, size_t start, size_t end)
{
    strncpy(result, str + start, end - start);
}

int feed() {
    while (1) {
        char SerialBuffer[128] = { 0 }; //Buffer to send and receive data
        DWORD dwEventMask = 0; // Event mask to trigger
        char ReadData; //temperory Character
        DWORD NoBytesRead; // Bytes read by ReadFile()
        unsigned char loop = 0;
        BOOL Status;
        if (SetCommMask(hSerial, EV_RXCHAR) == FALSE) {
            print_error("Failed to SetComm");
            CloseHandle(hSerial);
            return -1;
        }
        if (WaitCommEvent(hSerial, &dwEventMask, NULL) == FALSE) {
            print_error("Failed to WaitComm");
            CloseHandle(hSerial);
            return -1;
        }
        do {
            Status = ReadFile(hSerial, &ReadData, sizeof(ReadData), &NoBytesRead, NULL);
            SerialBuffer[loop] = ReadData;
            ++loop;
            if (SerialBuffer[loop - 1] == 0x3) {
                break;
            }
        } while (NoBytesRead > 0);

        --loop;
        printf_s("\nNumber of bytes received = %d\n\n", loop);
        printf_s("\n\n");
        int index = 0;
        for (index = 0; index < loop; ++index)
        {
            printf_s("%c", SerialBuffer[index]);
        }
        printf_s("\n\n");
        char channel = 0;
        char mac[64] = { 0 };

        // Extraire le canal
        
        if (SerialBuffer[2] == '\x1F') {
            channel = SerialBuffer[1] - '0';
        }
        else if (SerialBuffer[3] == '\x1F') {
            ;
        }
        //
        printf_s("%d", channel);
    }

    return 0;
}

int main(int argc, char* argv[]) {
    uint32_t baud_rate = NODEMCU_BAUD_RATE;
    SetConsoleCtrlHandler(ExitHandler, TRUE);
    openSerialPort(baud_rate);
    if (hSerial == INVALID_HANDLE_VALUE) { return 1; }

    feed();

    return 0;
}