#pragma once

#include "pch.h"
#include "ak_log.h"
// The indexes of each of the fields in the credential provider's tiles.
enum FIELD_ID {
  FI_TILEIMAGE = 0,
  FI_LABEL = 1,
  FI_LARGE_TEXT = 2,
  FI_PASSWORD = 3,
  FI_SUBMIT_BUTTON = 4,
  FI_LAUNCHWINDOW_LINK = 5,
  FI_HIDECONTROLS_LINK = 6,
  FI_FULLNAME_TEXT = 7,
  FI_DISPLAYNAME_TEXT = 8,
  FI_LOGONSTATUS_TEXT = 9,
  FI_CHECKBOX = 10,
  FI_EDIT_TEXT = 11,
  FI_COMBOBOX = 12,
  FI_NUM_FIELDS = 13,  // Note: if new fields are added, keep NUM_FIELDS last.  This is used as a
                       // count of the number of fields
};

// The first value indicates when the tile is displayed (selected, not selected)
// the second indicates things like whether the field is enabled, whether it has key focus, etc.
struct FIELD_STATE_PAIR {
  CREDENTIAL_PROVIDER_FIELD_STATE cpfs;
  CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE cpfis;
};

// These two arrays are seperate because a credential provider might
// want to set up a credential with various combinations of field state pairs
// and field descriptors.

// The field state value indicates whether the field is displayed
// in the selected tile, the deselected tile, or both.
// The Field interactive state indicates when
static const FIELD_STATE_PAIR s_rgFieldStatePairs[] = {
    {CPFS_DISPLAY_IN_BOTH, CPFIS_NONE},           // FI_TILEIMAGE
    {CPFS_HIDDEN, CPFIS_NONE},                    // FI_LABEL
    {CPFS_DISPLAY_IN_BOTH, CPFIS_NONE},           // FI_LARGE_TEXT
    {CPFS_HIDDEN, CPFIS_NONE},                    // FI_PASSWORD
    {CPFS_DISPLAY_IN_SELECTED_TILE, CPFIS_NONE},  // FI_SUBMIT_BUTTON
    {CPFS_HIDDEN, CPFIS_NONE},                    // FI_LAUNCHWINDOW_LINK
    {CPFS_HIDDEN, CPFIS_NONE},                    // FI_HIDECONTROLS_LINK
    {CPFS_HIDDEN, CPFIS_READONLY},                // FI_FULLNAME_TEXT
    {CPFS_HIDDEN, CPFIS_FOCUSED},                 // FI_DISPLAYNAME_TEXT
    {CPFS_HIDDEN, CPFIS_FOCUSED},                 // FI_LOGONSTATUS_TEXT
    {CPFS_HIDDEN, CPFIS_FOCUSED},                 // FI_CHECKBOX
    {CPFS_HIDDEN, CPFIS_NONE},                    // FI_EDIT_TEXT
    {CPFS_HIDDEN, CPFIS_NONE},                    // FI_COMBOBOX
};

// Field descriptors for unlock and logon.
// The first field is the index of the field.
// The second is the type of the field.
// The third is the name of the field, NOT the value which will appear in the field.
static const CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR s_rgCredProvFieldDescriptors[] = {
    {FI_TILEIMAGE, CPFT_TILE_IMAGE, const_cast<LPWSTR>(L"Image"), CPFG_CREDENTIAL_PROVIDER_LOGO},
    {FI_LABEL, CPFT_SMALL_TEXT, const_cast<LPWSTR>(L"Tooltip"), CPFG_CREDENTIAL_PROVIDER_LABEL},
    {FI_LARGE_TEXT, CPFT_LARGE_TEXT, const_cast<LPWSTR>(L"Sign in with authentik")},
    {FI_PASSWORD, CPFT_PASSWORD_TEXT, const_cast<LPWSTR>(L"Password text")},
    {FI_SUBMIT_BUTTON, CPFT_SUBMIT_BUTTON, const_cast<LPWSTR>(L"Submit"),
     CPFG_STANDALONE_SUBMIT_BUTTON},
    {FI_LAUNCHWINDOW_LINK, CPFT_COMMAND_LINK, const_cast<LPWSTR>(L"Launch helper window2")},
    {FI_HIDECONTROLS_LINK, CPFT_COMMAND_LINK, const_cast<LPWSTR>(L"Hide additional controls")},
    {FI_FULLNAME_TEXT, CPFT_SMALL_TEXT, const_cast<LPWSTR>(L"Full name: ")},
    {FI_DISPLAYNAME_TEXT, CPFT_SMALL_TEXT, const_cast<LPWSTR>(L"Display name: ")},
    {FI_LOGONSTATUS_TEXT, CPFT_SMALL_TEXT, const_cast<LPWSTR>(L"Logon status: ")},
    {FI_CHECKBOX, CPFT_CHECKBOX, const_cast<LPWSTR>(L"Checkbox")},
    {FI_EDIT_TEXT, CPFT_EDIT_TEXT, const_cast<LPWSTR>(L"Edit text")},
    {FI_COMBOBOX, CPFT_COMBOBOX, const_cast<LPWSTR>(L"Combobox")},
};

static const PWSTR s_rgComboBoxStrings[] = {
    const_cast<LPWSTR>(L"First"),
    const_cast<LPWSTR>(L"Second"),
    const_cast<LPWSTR>(L"Third"),
};
