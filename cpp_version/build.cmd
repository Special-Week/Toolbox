@echo off
chcp 65001 >nul
setlocal
pushd "%~dp0" || exit /b 1
rem 默认生成 x64；构建 x86 时，先将 PATH 切换到 mingw32，再执行 build.cmd x86。
set "ARCH=%~1"
if not defined ARCH set "ARCH=x64"
set "CPU_FLAGS="
if /i "%ARCH%"=="x64" (
    set "ARCH=x64"
    set "EXPECTED=x86_64"
) else if /i "%ARCH%"=="x86" (
    set "ARCH=x86"
    set "EXPECTED=i686"
    rem 32 位使用 SSE2 保持双精度计算行为，并静态链接 POSIX 工具链的线程库。
    set "CPU_FLAGS=-msse2 -mfpmath=sse -pthread"
) else (
    echo 用法：build.cmd [x64^|x86]
    goto failed
)
rem gcc 与 windres 须来自 PATH 中同一套 MinGW 工具链，防止混用位数。
set "TARGET="
for /f %%T in ('gcc -dumpmachine 2^>nul') do set "TARGET=%%T"
if not defined TARGET goto failed
echo %TARGET% | findstr /b /c:"%EXPECTED%-" >nul || (
    echo 当前 GCC 为 %TARGET%，请将 PATH 切换到 %ARCH% 工具链。
    goto failed
)
if not exist build mkdir build || goto failed
if not exist "dist\%ARCH%" mkdir "dist\%ARCH%" || goto failed
windres -c 65001 -O coff resources/Toolbox.rc -o build/Toolbox.res.o || goto failed
rem EXE 直接生成到发布目录，build 仅存放资源编译的中间文件。
gcc "@build.rsp" -O2 -s %CPU_FLAGS% -o "dist/%ARCH%/Toolbox.exe" || goto failed
echo 已生成：dist\%ARCH%\Toolbox.exe
popd
exit /b 0

:failed
echo 构建失败，请查看上方错误，并确认 gcc、g++、windres 可用。
popd
exit /b 1
