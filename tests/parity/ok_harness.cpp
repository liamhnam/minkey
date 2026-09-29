// Reference harness for tests/openkey_parity_test.rs: drives the original OpenKey engine
// (built with Win32 key codes). Input line: "<cfg digits>|<keys>", "@" = backspace. See regen.sh.
#include <iostream>
#include <sstream>
#include <string>
#include "Engine.h"
using namespace std;

int vLanguage = 1, vInputType = 0, vFreeMark = 0, vCodeTable = 0, vSwitchKeyStatus = 0, vCheckSpelling = 1,
    vUseModernOrthography = 0, vQuickTelex = 0, vRestoreIfWrongSpelling = 1, vFixRecommendBrowser = 0,
    vUseMacro = 0, vUseMacroInEnglishMode = 0, vAutoCapsMacro = 0, vUseSmartSwitchKey = 0,
    vUpperCaseFirstChar = 0, vTempOffSpelling = 0, vAllowConsonantZFWJ = 0, vQuickStartConsonant = 0,
    vQuickEndConsonant = 0, vRememberCode = 0, vOtherLanguage = 0, vTempOffOpenKey = 0;

static void mapChar(char32_t ch, Uint16& key, bool& caps) {
    caps = false;
    if (ch >= 'a' && ch <= 'z') { key = ch - 32; return; }
    if (ch >= 'A' && ch <= 'Z') { key = ch; caps = true; return; }
    if (ch >= '0' && ch <= '9') { key = ch; return; }
    switch (ch) {
        case ' ': key = KEY_SPACE; return;
        case '@': key = KEY_DELETE; return; // backspace token
        case '[': key = KEY_LEFT_BRACKET; return; case '{': key = KEY_LEFT_BRACKET; caps = true; return;
        case ']': key = KEY_RIGHT_BRACKET; return; case '}': key = KEY_RIGHT_BRACKET; caps = true; return;
        case '.': key = KEY_DOT; return; case ',': key = KEY_COMMA; return;
        case ';': key = KEY_SEMICOLON; return; case '\'': key = KEY_QUOTE; return;
        case '/': key = KEY_SLASH; return; case '-': key = KEY_MINUS; return;
        case '?': key = KEY_SLASH; caps = true; return; case '!': key = KEY_1; caps = true; return;
    }
    key = KEY_SPACE;
}

static void putUtf8(u32string& out, string& res) {
    wstring_convert<codecvt_utf8<char32_t>, char32_t> cv;
    res = cv.to_bytes(out);
}

int main() {
    vKeyHookState* pData = (vKeyHookState*)vKeyInit();
    string line;
    wstring_convert<codecvt_utf8<char32_t>, char32_t> cv;
    while (getline(cin, line)) {
        size_t bar = line.find('|');
        istringstream cfg(line.substr(0, bar));
        cfg >> vInputType >> vUseModernOrthography >> vCheckSpelling >> vRestoreIfWrongSpelling >> vQuickTelex
            >> vAllowConsonantZFWJ >> vQuickStartConsonant >> vQuickEndConsonant >> vFreeMark;
        vSetCheckSpelling();
        u32string text = cv.from_bytes(line.substr(bar + 1)), out;
        startNewSession();
        vKeyHandleEvent(vKeyEvent::Mouse, vKeyEventState::MouseDown, 0);
        for (char32_t ch : text) {
            Uint16 key; bool caps;
            mapChar(ch, key, caps);
            vKeyHandleEvent(vKeyEvent::Keyboard, vKeyEventState::KeyDown, key, caps ? 1 : 0, false);
            int code = pData->code;
            if (code == vDoNothing || code == vBreakWord) {
                if (key == KEY_DELETE) { if (!out.empty()) out.pop_back(); }
                else out.push_back(ch);
            } else if (code == vWillProcess || code == vRestore || code == vRestoreAndStartNewSession) {
                for (int i = 0; i < pData->backspaceCount && !out.empty(); i++) out.pop_back();
                for (int i = pData->newCharCount - 1; i >= 0; i--) {
                    Uint32 d = pData->charData[i];
                    if (d & PURE_CHARACTER_MASK) out.push_back(d & 0xFFFF);
                    else if (d & CHAR_CODE_MASK) out.push_back(d & 0xFFFF);
                    else out.push_back(keyCodeToCharacter(d));
                }
                if (code != vWillProcess) out.push_back(ch);
                if (code == vRestoreAndStartNewSession) startNewSession();
            }
        }
        cout << cv.to_bytes(out) << "\n";
    }
}
