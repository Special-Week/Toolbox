@echo off
chcp 65001 >nul
setlocal
pushd "%~dp0" || exit /b 1
set "TOOLBOX_ARCH=%~1"
if not defined TOOLBOX_ARCH set "TOOLBOX_ARCH=x64"
if /i "%TOOLBOX_ARCH%"=="x64" set "TOOLBOX_ARCH=x64"
if /i "%TOOLBOX_ARCH%"=="x86" set "TOOLBOX_ARCH=x86"
if not "%TOOLBOX_ARCH%"=="x64" if not "%TOOLBOX_ARCH%"=="x86" (
    echo 用法：build.cmd x64 或 build.cmd x86
    popd
    exit /b 1
)
where mingw32-make >nul 2>nul || (
    echo 未找到 mingw32-make，请将目标架构的 MinGW 工具链加入 PATH。
    popd
    exit /b 1
)
if not defined TOOLBOX_BUILD_JOBS set "TOOLBOX_BUILD_JOBS=4"
mingw32-make -j%TOOLBOX_BUILD_JOBS% ARCH=%TOOLBOX_ARCH% all
set "TOOLBOX_BUILD_RESULT=%ERRORLEVEL%"
popd
exit /b %TOOLBOX_BUILD_RESULT%
