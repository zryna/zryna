@echo off
setlocal
if "%~1"=="" exit /b 2
if not "%~2"=="" exit /b 2
if not "%~1"=="%~f1" exit /b 2
if exist "%~1" exit /b 2
mkdir "%~1" || exit /b 2
set "WA_BUILD_DIR=%~1"
set "CL="
set "_CL_="
set "LINK="
set "_LINK_="
pushd "%~dp0" || exit /b 2
cl /nologo /std:c17 /W4 /WX /O2 /MT /GS /guard:cf /Brepro /utf-8 /DUNICODE /D_UNICODE /D_WIN32_WINNT=0x0A00 /Fo"%WA_BUILD_DIR%\\" /Fe"%WA_BUILD_DIR%\windows-parent-affinity-v1.exe" probe.c /link /Brepro /NXCOMPAT /DYNAMICBASE kernel32.lib
set "WA_BUILD_EXIT=%ERRORLEVEL%"
popd
exit /b %WA_BUILD_EXIT%
