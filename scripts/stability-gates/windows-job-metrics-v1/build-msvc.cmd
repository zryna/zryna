@echo off
setlocal
if "%~1"=="" exit /b 2
if not "%~2"=="" exit /b 2
if not "%~1"=="%~f1" exit /b 2
if exist "%~1" exit /b 2
mkdir "%~1" || exit /b 2
set "WM_BUILD_DIR=%~1"
set "CL="
set "_CL_="
set "LINK="
set "_LINK_="
pushd "%~dp0" || exit /b 2
cl /nologo /std:c17 /W4 /WX /O2 /MT /GS /guard:cf /Brepro /utf-8 /DUNICODE /D_UNICODE /D_WIN32_WINNT=0x0A00 /Fo"%WM_BUILD_DIR%\\" /Fe"%WM_BUILD_DIR%\windows-job-metrics-v1.exe" portable.c options.c pipes.c streams.c launch.c measure.c cleanup.c result.c main.c /link /Brepro /NXCOMPAT /DYNAMICBASE kernel32.lib
set "WM_BUILD_EXIT=%ERRORLEVEL%"
popd
exit /b %WM_BUILD_EXIT%
