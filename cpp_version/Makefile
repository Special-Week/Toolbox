# Windows 7 目标；每种架构使用独立对象与资源，避免切换工具链时混用。
SHELL := cmd.exe
CC := gcc
CXX := g++
WINDRES := windres
ARCH ?= x64
TRIPLE := $(strip $(shell $(CC) -dumpmachine))

ifeq ($(ARCH),x86)
  ifeq ($(filter i686-%,$(TRIPLE)),)
    $(error ARCH=x86 需要 i686 MinGW，当前编译器为 $(TRIPLE))
  endif
  CPU_FLAGS := -msse2 -mfpmath=sse -pthread
else ifeq ($(ARCH),x64)
  ifeq ($(filter x86_64-%,$(TRIPLE)),)
    $(error ARCH=x64 需要 x86_64 MinGW，当前编译器为 $(TRIPLE))
  endif
  CPU_FLAGS :=
else
  $(error ARCH 只能是 x86 或 x64)
endif

BUILD := build/$(ARCH)
comma := ,
# 与 Windows 资源中的数字版本保持一致；第四段为零时省略。
VERSION_RAW := $(word 2,$(shell findstr FILEVERSION resources\Toolbox.rc))
VERSION := $(patsubst %.0,%,$(subst $(comma),.,$(VERSION_RAW)))
ifeq ($(VERSION),)
  $(error 无法从 resources/Toolbox.rc 读取 FILEVERSION)
endif
TARGET := dist/$(ARCH)/Toolbox_cpp_$(ARCH)_v$(VERSION).exe
CPP_SOURCES := $(filter %.cpp %.cc,$(file <build.rsp)) $(file <microtex.rsp)
C_SOURCES := $(wildcard third_party/pdfio/pdfio-*.c third_party/pdfio/ttf/*.c third_party/zlib/*.c)
CPP_OBJECTS := $(addprefix $(BUILD)/,$(addsuffix .o,$(CPP_SOURCES)))
C_OBJECTS := $(addprefix $(BUILD)/,$(addsuffix .o,$(C_SOURCES)))
OBJECTS := $(CPP_OBJECTS) $(C_OBJECTS)
RESOURCE := $(BUILD)/Toolbox.res.o
INCLUDES := -Iinclude -isystem third_party -isystem third_party/pdfio -isystem third_party/pdfio/ttf -isystem third_party/zlib -isystem third_party/microtex/src -isystem third_party/tinyxml2
DEFINES := -DUNICODE -D_UNICODE -D_WIN32_WINNT=0x0601 -DWINVER=0x0601 -DBUILD_WIN32 -DNDEBUG
CXXFLAGS := -std=c++17 -O2 -Wall -Wextra -Wpedantic -finput-charset=UTF-8 -fexec-charset=UTF-8 -fno-fast-math -ffp-contract=off $(DEFINES) $(INCLUDES) $(CPU_FLAGS)
# Keep strict diagnostics for the application; the bundled MicroTeX snapshot has legacy warnings.
MICROTEX_CXXFLAGS := $(CXXFLAGS) -w
CFLAGS := -std=c11 -O2 -w -D_WIN32_WINNT=0x0601 -DWINVER=0x0601 $(INCLUDES)
LIBS := -lstdc++ -ld2d1 -ldwrite -lwindowscodecs -lcomctl32 -lcomdlg32 -lole32 -lgdi32 -luuid -lwinhttp -lbcrypt -ladvapi32 -lwinpthread -lshell32 -lgdiplus
LINKFLAGS := -O2 -s $(CPU_FLAGS) -municode -mwindows -static -static-libgcc -Wl,--major-subsystem-version,6,--minor-subsystem-version,1
RESOURCE_DEPS := resources/Toolbox.rc resources/microtex-fonts.rcinc include/toolbox/resource_ids.h $(wildcard resources/*.ico resources/*.manifest resources/*.json resources/gauges/*.json licenses/*.txt third_party/microtex/res/fonts/*/*.ttf third_party/microtex/res/fonts/*/*/*.ttf)

.PHONY: all
.DELETE_ON_ERROR:
all: $(TARGET)

# 并发创建目录时，只有目录仍不存在才视为失败。
define ensure_dir
@if not exist "$(subst /,\,$(1))" mkdir "$(subst /,\,$(1))" 2>nul || if not exist "$(subst /,\,$(1))" exit /b 1
endef

$(BUILD)/third_party/microtex/%.cpp.o: third_party/microtex/%.cpp Makefile microtex.rsp
	$(call ensure_dir,$(@D))
	$(CXX) $(MICROTEX_CXXFLAGS) -MMD -MP -c "$<" -o "$@"

$(BUILD)/%.cpp.o: %.cpp Makefile build.rsp microtex.rsp
	$(call ensure_dir,$(@D))
	$(CXX) $(CXXFLAGS) -MMD -MP -c "$<" -o "$@"

$(BUILD)/%.cc.o: %.cc Makefile build.rsp microtex.rsp
	$(call ensure_dir,$(@D))
	$(CXX) $(CXXFLAGS) -MMD -MP -c "$<" -o "$@"

$(BUILD)/%.c.o: %.c Makefile
	$(call ensure_dir,$(@D))
	$(CC) $(CFLAGS) -MMD -MP -c "$<" -o "$@"

$(RESOURCE): $(RESOURCE_DEPS) Makefile
	$(call ensure_dir,$(@D))
	$(WINDRES) -c 65001 -O coff resources/Toolbox.rc -o "$@"

$(TARGET): $(OBJECTS) $(RESOURCE) Makefile
	$(call ensure_dir,$(@D))
	$(file >$(BUILD)/link.rsp,$(OBJECTS) $(RESOURCE) $(LINKFLAGS) $(LIBS))
	$(CC) @$(BUILD)/link.rsp -o "$@"
	@echo 已生成：$(TARGET)

-include $(OBJECTS:.o=.d)
