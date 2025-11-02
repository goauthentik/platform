#
# This python patch script automatically adds the Visual Studio Setup project to
# the CMake generated Visual Studio solution file (.sln) since the VS Setup project
# is not based on CMake.
# Usage:    Launch this script after the CMake configuration step from the CMake binary
# directory containing the Visual Studio solution file (.sln)
#
# Example:  python ../.utils/addsetup.py
#

# import uuid
# struuid = str(uuid.uuid4())
import os, sys

def find_and_insert(str, key, arrIns):
    keypos = str.rfind(key)
    keypos = keypos + len(key)
    keypostmp = keypos
    spaces = ""
    while str[keypostmp] == "\t":
        keypostmp = keypostmp + 1
        spaces = spaces + "\t"
    strIns = ""
    for item in arrIns:
        strIns = strIns + spaces + item + """
"""
    return (str[0:keypos] + strIns + str[keypos:])

def get_common_uuid(str, key):
    halfstr = str[0:str.find(key)]
    return halfstr[(halfstr.rfind("{") + 1):(halfstr.rfind("}"))]

def get_project_uuid(str, key):
    halfstr = str[str.find(key):]
    return halfstr[(halfstr.find("{") + 1):(halfstr.find("}"))]

def find_and_replace_line(str, key, arrIns):
    keypos = str.rfind(key)
    keypos = keypos + len(key)
    keypostmp = keypos
    spaces = ""
    while str[keypostmp] == "\t":
        keypostmp = keypostmp + 1
        spaces = spaces + "\t"
    prestr = str[0:keypos]
    prestr = prestr[0:(prestr.rfind("\n") + 1)]
    poststr = str[keypos:]
    poststr = poststr[(poststr.find("\n")+1):]
    strIns = ""
    for item in arrIns:
        strIns = strIns + spaces + item + """
"""
    return (prestr + strIns + poststr)

# main script
uuid_libcef = ""
fsln = os.path.join(os.getcwd(), "cef.sln")
if os.path.isfile(fsln):
    hfsln = open(fsln, "r")
    str = hfsln.read()
    hfsln.close()
    uuid_libcef = get_project_uuid(str, "libcef_dll_wrapper")
    if str.find("Setup.vdproj") >= 0:
        print(f"Solution file `{fsln}` is already patched. Skip.")
    else:
        print(f"Solution file `{fsln}` being patched...")
        uuid_common = get_common_uuid(str, "ALL_BUILD")
        uuid_depend = get_project_uuid(str, "ALL_BUILD")
        # insert `Setup` project entry
        key = """EndProject
"""
        arrIns = [
            "Project(\"{54435603-DBB4-11D2-8724-00A0C9A8B90C}\") = \"Setup\", \".\\Setup\\Setup.vdproj\", \"{8818B3B0-D7DD-4294-A5C0-29BA5DE85BEE}\"",
                "\tProjectSection(ProjectDependencies) = postProject",
                    "\t\t{" + uuid_depend + "} = {" + uuid_depend + "}",
                "\tEndProjectSection",
            "EndProject"
        ]
        str = find_and_insert(str, key, arrIns)

        # insert `Setup` project entry
        key = """GlobalSection(ProjectConfigurationPlatforms) = postSolution
"""
        arrIns = [
            "{8818B3B0-D7DD-4294-A5C0-29BA5DE85BEE}.Debug|x64.ActiveCfg = Debug", # Debug|x64
            "{8818B3B0-D7DD-4294-A5C0-29BA5DE85BEE}.Debug|x64.Build.0 = Debug", # Debug|x64
            "{8818B3B0-D7DD-4294-A5C0-29BA5DE85BEE}.Release|x64.ActiveCfg = Release", # Release|x64
            "{8818B3B0-D7DD-4294-A5C0-29BA5DE85BEE}.Release|x64.Build.0 = Release", # Release|x64
        ]
        str = find_and_insert(str, key, arrIns)

        # write back the patched contents to file
        hfsln = open(fsln, "w")
        hfsln.write(str)
        hfsln.close()
        print(f"Solution file `{fsln}` patched.")
else:
    print(f"Error: Solution file path `{fsln}` is not valid.")
    sys.exit(1)

file = os.path.join(os.getcwd(), "Setup\\Setup.vdproj")
if os.path.isfile(file):
    hfile = open(file, "r")
    str = hfile.read()
    hfile.close()
    if str.find("\"OutputProjectGuid\" = \"8:{" + uuid_libcef + "}\"") >= 0:
        print(f"Setup file `{file}` is already patched. Skip.")
    else:
        print(f"Solution file `{file}` being patched...")
        # replace guid of (to be packaged) output project
        key = "OutputProjectGuid"
        arrIns = [
            "\t\t\t\"OutputProjectGuid\" = \"8:{" + uuid_libcef + "}\""
        ]
        str = find_and_replace_line(str, key, arrIns)

        # write back the patched contents to file
        hfile = open(file, "w")
        hfile.write(str)
        hfile.close()
        print(f"Setup file `{file}` patched.")
else:
    print(f"Error: Setup file path `{file}` is not valid.")
    sys.exit(1)
