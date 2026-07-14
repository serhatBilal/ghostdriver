cmake_minimum_required(VERSION 3.24)
project(ghost_cuda_probe LANGUAGES CXX)

find_package(CUDAToolkit REQUIRED)

add_executable(ghost-cuda-probe main.cpp)
target_compile_features(ghost-cuda-probe PRIVATE cxx_std_17)
target_link_libraries(ghost-cuda-probe PRIVATE CUDA::cuda_driver)
set_target_properties(
    ghost-cuda-probe
    PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}"
)
foreach(CONFIGURATION Debug Release RelWithDebInfo MinSizeRel)
    string(TOUPPER "${CONFIGURATION}" CONFIGURATION_UPPER)
    set_target_properties(
        ghost-cuda-probe
        PROPERTIES
            "RUNTIME_OUTPUT_DIRECTORY_${CONFIGURATION_UPPER}"
            "${CMAKE_CURRENT_BINARY_DIR}"
    )
endforeach()

if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(ghost-cuda-probe PRIVATE -Wall -Wextra -Wpedantic -Werror)
endif()

set(GHOST_FATBIN "${CMAKE_CURRENT_BINARY_DIR}/ghost_cuda_probe.fatbin")
add_custom_command(
    OUTPUT "${GHOST_FATBIN}"
    COMMAND
        "${CUDAToolkit_NVCC_EXECUTABLE}"
        --fatbin
        --std=c++17
        --generate-code=arch=compute_75,code=sm_75
        "${CMAKE_CURRENT_SOURCE_DIR}/vector_add.cu"
        -o "${GHOST_FATBIN}"
    DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/vector_add.cu"
    COMMENT "Building deterministic SM 7.5 fatbin"
    VERBATIM
)
add_custom_target(ghost-cuda-fatbin ALL DEPENDS "${GHOST_FATBIN}")
add_dependencies(ghost-cuda-probe ghost-cuda-fatbin)
