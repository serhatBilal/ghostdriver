#include <cuda.h>

#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

void check(CUresult result, const char* operation) {
    if (result == CUDA_SUCCESS) {
        return;
    }

    const char* name = nullptr;
    const char* description = nullptr;
    cuGetErrorName(result, &name);
    cuGetErrorString(result, &description);

    throw std::runtime_error(
        std::string(operation) + " failed: " +
        (name ? name : "UNKNOWN") + " - " +
        (description ? description : "no description")
    );
}

int parse_threads(int argc, char** argv) {
    int threads = 32;
    for (int i = 1; i < argc; ++i) {
        const std::string argument = argv[i];
        if (argument == "--threads" && i + 1 < argc) {
            threads = std::stoi(argv[++i]);
        } else if (argument == "--help") {
            std::cout << "Usage: ghost-cuda-probe [--threads 32|64|128|256]\n";
            std::exit(EXIT_SUCCESS);
        } else {
            throw std::invalid_argument("unknown argument: " + argument);
        }
    }

    if (threads <= 0 || threads > 1024) {
        throw std::invalid_argument("threads must be in the range 1..1024");
    }

    return threads;
}

std::filesystem::path module_path(const char* argv0) {
    const auto executable = std::filesystem::canonical(argv0);
    return executable.parent_path() / "ghost_cuda_probe.fatbin";
}

}  // namespace

int main(int argc, char** argv) {
    try {
        const int threads = parse_threads(argc, argv);
        constexpr int count = 4096;
        int kernel_count = count;
        const std::size_t bytes = count * sizeof(float);

        check(cuInit(0), "cuInit");

        CUdevice device{};
        check(cuDeviceGet(&device, 0), "cuDeviceGet");

        char device_name[256]{};
        check(cuDeviceGetName(device_name, sizeof(device_name), device), "cuDeviceGetName");

        CUcontext context{};
        check(cuCtxCreate(&context, 0, device), "cuCtxCreate");

        CUmodule module{};
        const auto fatbin = module_path(argv[0]);
        check(cuModuleLoad(&module, fatbin.c_str()), "cuModuleLoad");

        CUfunction function{};
        check(cuModuleGetFunction(&function, module, "vector_add"), "cuModuleGetFunction");

        std::vector<float> host_a(count);
        std::vector<float> host_b(count);
        std::vector<float> host_c(count, 0.0F);

        for (int i = 0; i < count; ++i) {
            host_a[i] = static_cast<float>(i) * 0.5F;
            host_b[i] = static_cast<float>(i) * 0.25F;
        }

        CUdeviceptr device_a{};
        CUdeviceptr device_b{};
        CUdeviceptr device_c{};
        check(cuMemAlloc(&device_a, bytes), "cuMemAlloc(a)");
        check(cuMemAlloc(&device_b, bytes), "cuMemAlloc(b)");
        check(cuMemAlloc(&device_c, bytes), "cuMemAlloc(c)");

        check(cuMemcpyHtoD(device_a, host_a.data(), bytes), "cuMemcpyHtoD(a)");
        check(cuMemcpyHtoD(device_b, host_b.data(), bytes), "cuMemcpyHtoD(b)");

        void* parameters[] = {
            &device_a,
            &device_b,
            &device_c,
            &kernel_count,
        };

        const unsigned int blocks =
            static_cast<unsigned int>((count + threads - 1) / threads);

        check(
            cuLaunchKernel(
                function,
                blocks, 1, 1,
                static_cast<unsigned int>(threads), 1, 1,
                0,
                nullptr,
                parameters,
                nullptr
            ),
            "cuLaunchKernel"
        );

        check(cuCtxSynchronize(), "cuCtxSynchronize");
        check(cuMemcpyDtoH(host_c.data(), device_c, bytes), "cuMemcpyDtoH(c)");

        bool valid = true;
        for (int i = 0; i < count; ++i) {
            const float expected = host_a[i] + host_b[i];
            if (std::fabs(host_c[i] - expected) > 1e-5F) {
                valid = false;
                std::cerr << "mismatch index=" << i
                          << " expected=" << expected
                          << " actual=" << host_c[i] << '\n';
                break;
            }
        }

        std::cout << "device=" << device_name << '\n';
        std::cout << "elements=" << count << '\n';
        std::cout << "threads=" << threads << '\n';
        std::cout << "blocks=" << blocks << '\n';
        std::cout << "verification=" << (valid ? "passed" : "failed") << '\n';

        cuMemFree(device_c);
        cuMemFree(device_b);
        cuMemFree(device_a);
        cuModuleUnload(module);
        cuCtxDestroy(context);

        return valid ? EXIT_SUCCESS : EXIT_FAILURE;
    } catch (const std::exception& error) {
        std::cerr << "error=" << error.what() << '\n';
        return EXIT_FAILURE;
    }
}
