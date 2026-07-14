if(NOT DEFINED CMAKE_CUDA_COMPILER)
    message(FATAL_ERROR "CMAKE_CUDA_COMPILER is required")
endif()

execute_process(
    COMMAND
        "${CMAKE_CUDA_COMPILER}"
        --fatbin
        "${CMAKE_CURRENT_LIST_DIR}/vector_add.cu"
        -o
        "${CMAKE_CURRENT_BINARY_DIR}/ghost_cuda_probe.fatbin"
    COMMAND_ERROR_IS_FATAL ANY
)
