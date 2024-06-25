#
# Copyright 2023, Colias Group, LLC
#
# SPDX-License-Identifier: BSD-2-Clause
#

FROM trustworthysystems/sel4

WORKDIR /tmp

RUN rm -r *

RUN apt-get update -q && apt-get install -y --no-install-recommends \
    bash-completion man sudo \
    && rm -rf /var/lib/apt/lists/*

RUN curl -sSf https://sh.rustup.rs | \
        bash -s -- -y --no-modify-path \
            --default-toolchain nightly-2024-05-01 \
            --component rust-src

ENV PATH=/root/.cargo/bin:$PATH

ENV SEL4_INSTALL_DIR=/opt/seL4

RUN set -eux; \
    git clone \
        https://github.com/seL4/seL4.git \
        --config advice.detachedHead=false; \
    cd seL4; \
    git checkout 0260fba168b74dd2504c9447f2fa8da1161ead87;

COPY kernel-settings.cmake .
RUN apt-get update -q && apt-get install -y --no-install-recommends gcc-riscv64-linux-gnu && rm -rf /var/lib/apt/lists/*
    
# RUN set -eux; \
#     cd seL4; \
#     cmake \
#         -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
#         -DCMAKE_INSTALL_PREFIX=$SEL4_INSTALL_DIR \
#         -DKernelPlatform=qemu-arm-virt \
#         # TODO further develop kernel loader to make KernelArmHypervisorSupport=ON unecessary
#         -DKernelArmHypervisorSupport=ON \
#         -DKernelVerificationBuild=OFF \
#         -DARM_CPU=cortex-a57 \
#         -G Ninja \
#         -S . \
#         -B build; \
#     ninja -C build all; \
#     ninja -C build install; \
#     rm -rf $(pwd);

RUN set -eux; \
    cd seL4; \
    cmake \
        -DCROSS_COMPILER_PREFIX=riscv64-linux-gnu- \
        -DCMAKE_INSTALL_PREFIX=$SEL4_INSTALL_DIR \
        -DKernelPlatform=spike \
        -C ../kernel-settings.cmake \
        -G Ninja \
        -S . \
        -B build; \
    ninja -C build all; \
    ninja -C build install; \
    rm -rf $(pwd);

# RUN set -eux; \
#     url="https://github.com/seL4/rust-sel4"; \
#     rev="a20aab14c0533d3145a2db75020e60151ce9b78f"; \
#     common_args="--git $url --rev $rev --root $SEL4_INSTALL_DIR"; \
#     CC_aarch64_unknown_none=aarch64-linux-gnu-gcc \
#     SEL4_PREFIX=$SEL4_INSTALL_DIR \
#         cargo install \
#             -Z build-std=core,compiler_builtins \
#             -Z build-std-features=compiler-builtins-mem \
#             --target aarch64-unknown-none \
#             $common_args \
#             sel4-kernel-loader; \
#     cargo install \
#         $common_args \
#         sel4-kernel-loader-add-payload;
RUN set -eux; \
    url="https://github.com/seL4/rust-sel4"; \
    rev="a20aab14c0533d3145a2db75020e60151ce9b78f"; \
    common_args="--git $url --rev $rev --root $SEL4_INSTALL_DIR"; \
    CC=riscv64-linux-gnu-gcc \
    SEL4_PREFIX=$SEL4_INSTALL_DIR \
        cargo install \
            -Z build-std=core,compiler_builtins \
            -Z build-std-features=compiler-builtins-mem \
            --target riscv64imac-unknown-none-elf \
            $common_args \
            sel4-kernel-loader; \
    cargo install \
        $common_args \
        sel4-kernel-loader-add-payload;

COPY lib_patch.patch .

# Clone rel4 kernel
RUN set -eux; \
        git clone \
            https://github.com/rel4team/rel4_kernel.git \
            --config advice.detachedHead=false; \
        cd rel4_kernel; \
        git checkout cf2cbf43ff6bbebd2a4390ec1ca9c1fa98085ba9; \
        git apply ../lib_patch.patch;
RUN set -eux; \
        git clone \
            https://github.com/rel4team/seL4_c_impl.git \
            --config advice.detachedHead=false; \
        cd seL4_c_impl; \
        git checkout 552f173d3d7780b33184ebedefc58329ea5de3ba;

RUN set -eux; \
    cd rel4_kernel;\
    rustup target add riscv64imac-unknown-none-elf; \
    make run;\
    cd ../seL4_c_impl; \
    cmake \
        -DCROSS_COMPILER_PREFIX=riscv64-linux-gnu- \
        -DCMAKE_INSTALL_PREFIX=/opt/reL4 \
        -DKernelPlatform=spike \
        -C ../kernel-settings.cmake \
        -G Ninja \
        -S . \
        -B build; \
    ninja -C build all; \
    ninja -C build install; \
    rm -rf $(pwd);

RUN cp /opt/seL4/libsel4/include/interfaces/*.xml /opt/reL4/libsel4/include/interfaces/
RUN cp /opt/seL4/bin/sel4-kernel-loader /opt/reL4/bin/
RUN cp /opt/seL4/bin/sel4-kernel-loader-add-payload /opt/reL4/bin/

ARG UID
ARG GID

RUN set -eux; \
    if [ $UID -eq 0 ]; then \
        if [ $GID -ne 0 ]; then \
            echo "error: \$UID == 0 but \$GID != 0" >&2; \
            exit 1; \
        fi; \
    else \
        if getent passwd $UID; then \
            echo "error: \$UID $UID already exists" >&2; \
            exit 1; \
        fi; \
        if ! getent group $GID; then \
            groupadd --gid $GID x; \
        fi; \
        useradd --uid $UID --gid $GID --groups sudo --create-home x; \
    fi;

RUN echo '%sudo ALL=(ALL) NOPASSWD:ALL' >> /etc/sudoers

USER $UID

RUN set -eux; \
    if [ $UID -ne 0 ]; then \
        curl -sSf https://sh.rustup.rs | \
            bash -s -- -y --no-modify-path --default-toolchain none; \
    fi;

ENV PATH=/home/x/.cargo/bin:$PATH

RUN rustup toolchain install nightly-2024-05-01-x86_64-unknown-linux-gnu
RUN rustup component add llvm-tools rust-src rustc-dev
RUN rustup target add riscv64imac-unknown-none-elf

WORKDIR /work
