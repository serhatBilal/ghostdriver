cmake_minimum_required(VERSION 3.24)
project(ghost_cuda_probe LANGUAGES CXX CUDA)

find_package(CUDAToolkit REQUIRED)

add_executable(ghost-cuda-probe main.cpp)
target_compile_features(ghost-cuda-probe PRIVATE cxx_std_17)
target_link_libraries(ghost-cuda-probe PRIVATE CUDA::cuda_driver)

add_custom_command(
    TARGET ghost-cuda-probe POST_BUILD
    COMMAND
        ${CMAKE_CUDA_COMPILER}
        --fatbin
        -gencode=arch=compute_75,code=sm_75
        ${CMAKE_CURRENT_SOURCE_DIR}/vector_add.cu
        -o
        $<TARGET_FILE_DIR:ghost-cuda-probe>/ghost_cuda_probe.fatbin
    VERBATIM
)
