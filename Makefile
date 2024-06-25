#
# Copyright 2023, Colias Group, LLC
#
# SPDX-License-Identifier: BSD-2-Clause
#

BUILD ?= build
OS    := sel4
build_dir := $(BUILD)

.PHONY: none
none:

.PHONY: clean
clean:
	rm -rf $(build_dir)

sel4_prefix := $(SEL4_INSTALL_DIR)

ifeq ($(OS), rel4)
export sel4_prefix = /opt/reL4
endif

# Kernel loader binary artifacts provided by Docker container:
# - `sel4-kernel-loader`: The loader binary, which expects to have a payload appended later via
#   binary patch.
# - `sel4-kernel-loader-add-payload`: CLI which appends a payload to the loader.
loader_artifacts_dir := $(sel4_prefix)/bin
loader := $(loader_artifacts_dir)/sel4-kernel-loader
loader_cli := $(loader_artifacts_dir)/sel4-kernel-loader-add-payload

app_crate := example
app := $(build_dir)/$(app_crate).elf

$(app): $(app).intermediate

# SEL4_TARGET_PREFIX is used by build.rs scripts of various rust-sel4 crates to locate seL4
# configuration and libsel4 headers.
.INTERMDIATE: $(app).intermediate
# $(app).intermediate:
# 	SEL4_PREFIX=$(sel4_prefix) \
# 		cargo build \
# 			-Z build-std=core,alloc,compiler_builtins \
# 			-Z build-std-features=compiler-builtins-mem \
# 			--target-dir $(build_dir)/target \
# 			--out-dir $(build_dir) \
# 			--target aarch64-sel4 \
# 			-p $(app_crate)
$(app).intermediate:
	SEL4_PREFIX=/opt/seL4 \
		cargo build \
			-Z build-std=core,alloc,compiler_builtins \
			-Z build-std-features=compiler-builtins-mem \
			--target-dir $(build_dir)/target \
			--out-dir $(build_dir) \
			--target riscv64imac-sel4 \
			-p $(app_crate)

image := $(build_dir)/image.elf

# Append the payload to the loader using the loader CLI
$(image): $(app) $(loader) $(loader_cli)
	$(loader_cli) \
		--loader $(loader) \
		--sel4-prefix $(sel4_prefix) \
		--app $(app) \
		-o $@

# qemu_cmd := \
# 	qemu-system-aarch64 \
# 		-machine virt,virtualization=on -cpu cortex-a57 -m size=1G \
# 		-serial mon:stdio \
# 		-nographic \
# 		-kernel $(image)
qemu_cmd := \
	qemu-system-riscv64 \
		-machine virt -m size=4G \
		-serial mon:stdio \
		-nographic \
		-kernel $(image) \
		-D qemu.log -d in_asm,int,pcall,cpu_reset,guest_errors

.PHONY: run
run: $(image)
	$(qemu_cmd)

.PHONY: test
test: test.py $(image)
	python3 $< $(qemu_cmd)

install:
	cd kernel/rel4_kernel && make run
	cd kernel/seL4_c_impl && cmake \
			-DCROSS_COMPILER_PREFIX=riscv64-linux-gnu- \
			-DCMAKE_INSTALL_PREFIX=/opt/reL4 \
			-DKernelPlatform=spike \
			-C ../kernel-settings.cmake \
			-G Ninja \
			-S . \
			-B build && \
    	ninja -C build clean && ninja -C build all && sudo ninja -C build install
#	sudo cp /opt/reL4/libsel4/include/interfaces/sel4-arch.xml /opt/reL4/libsel4/include/interfaces/object-api-arch.xml  
#	sudo cp /opt/reL4/libsel4/include/interfaces/sel4.xml /opt/reL4/libsel4/include/interfaces/object-api.xml  
#	sudo cp /opt/reL4/libsel4/include/interfaces/sel4-sel4arch.xml /opt/reL4/libsel4/include/interfaces/object-api-sel4-arch.xml  
	sudo cp /opt/seL4/libsel4/include/interfaces/*.xml /opt/reL4/libsel4/include/interfaces/
	sudo cp /opt/seL4/bin/sel4-kernel-loader /opt/reL4/bin/
	sudo cp /opt/seL4/bin/sel4-kernel-loader-add-payload /opt/reL4/bin/
.PHONY: install
