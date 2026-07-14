#include <cuda.h>

#include <array>
#include <charconv>
#include <cmath>
#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <stdexcept>
#include <string>
#include <system_error>
#include <vector>

namespace {

constexpr int kElementCount = 4096;
constexpr std::array<int, 4> kAllowedThreadCounts{32, 64, 128, 256};

std::string cuda_error(CUresult result) {
    const char* name = nullptr;
    const char* description = nullptr;
    cuGetErrorName(result, &name);
    cuGetErrorString(result, &description);
    return std::string(name ? name : "UNKNOWN") + " - " +
           (description ? description : "no description");
}

void check(CUresult result, const char* operation) {
    if (result != CUDA_SUCCESS) {
        throw std::runtime_error(
            std::string(operation) + " failed: " + cuda_error(result)
        );
    }
}

class CudaResources {
public:
    CudaResources() = default;
    CudaResources(const CudaResources&) = delete;
    CudaResources& operator=(const CudaResources&) = delete;

    ~CudaResources() {
        release_noexcept();
    }

    void release() {
        release_device(device_c, "cuMemFree(c)");
        release_device(device_b, "cuMemFree(b)");
        release_device(device_a, "cuMemFree(a)");
        if (module != nullptr) {
            check(cuModuleUnload(module), "cuModuleUnload");
            module = nullptr;
        }
        if (context != nullptr) {
            check(cuCtxDestroy(context), "cuCtxDestroy");
            context = nullptr;
        }
    }

    CUcontext context{};
    CUmodule module{};
    CUdeviceptr device_a{};
    CUdeviceptr device_b{};
    CUdeviceptr device_c{};

private:
    static void release_device(CUdeviceptr& pointer, const char* operation) {
        if (pointer != 0) {
            check(cuMemFree(pointer), operation);
            pointer = 0;
        }
    }

    void release_noexcept() noexcept {
        try {
            release();
        } catch (const std::exception& error) {
            std::cerr << "cleanup_error=" << error.what() << '\n';
        }
    }
};

int parse_integer(const std::string& value) {
    int parsed = 0;
    const auto result = std::from_chars(value.data(), value.data() + value.size(), parsed);
    if (result.ec != std::errc{} || result.ptr != value.data() + value.size()) {
        throw std::invalid_argument("threads must be an integer");
    }
    return parsed;
}

int parse_threads(int argc, char** argv) {
    int threads = 32;
    bool threads_seen = false;
    for (int i = 1; i < argc; ++i) {
        const std::string argument = argv[i];
        if (argument == "--threads") {
            if (threads_seen) {
                throw std::invalid_argument("--threads may only be specified once");
            }
            if (i + 1 >= argc) {
                throw std::invalid_argument("--threads requires a value");
            }
            threads = parse_integer(argv[++i]);
            threads_seen = true;
        } else if (argument == "--help") {
            std::cout << "Usage: ghost-cuda-probe [--threads 32|64|128|256]\n";
            std::exit(EXIT_SUCCESS);
        } else {
            throw std::invalid_argument("unknown argument: " + argument);
        }
    }

    bool allowed = false;
    for (const int candidate : kAllowedThreadCounts) {
        allowed = allowed || threads == candidate;
    }
    if (!allowed) {
        throw std::invalid_argument("threads must be one of 32, 64, 128, or 256");
    }
    return threads;
}

std::filesystem::path module_path(const char* argv0) {
    std::error_code error;
    auto executable = std::filesystem::read_symlink("/proc/self/exe", error);
    if (error) {
        error.clear();
        executable = std::filesystem::absolute(argv0, error);
    }
    if (error) {
        throw std::runtime_error("could not resolve probe executable path: " + error.message());
    }
    return executable.parent_path() / "ghost_cuda_probe.fatbin";
}

}  // namespace

int main(int argc, char** argv) {
    try {
        const int threads = parse_threads(argc, argv);
        const std::size_t bytes = kElementCount * sizeof(float);
        int kernel_count = kElementCount;

        check(cuInit(0), "cuInit");

        CUdevice device{};
        check(cuDeviceGet(&device, 0), "cuDeviceGet");

        char device_name[256]{};
        check(cuDeviceGetName(device_name, sizeof(device_name), device), "cuDeviceGetName");

        int compute_major = 0;
        int compute_minor = 0;
        check(
            cuDeviceComputeCapability(&compute_major, &compute_minor, device),
            "cuDeviceComputeCapability"
        );

        CudaResources resources;
        check(cuCtxCreate(&resources.context, 0, device), "cuCtxCreate");

        const auto fatbin = module_path(argv[0]);
        check(cuModuleLoad(&resources.module, fatbin.string().c_str()), "cuModuleLoad");

        CUfunction function{};
        check(
            cuModuleGetFunction(&function, resources.module, "vector_add"),
            "cuModuleGetFunction"
        );

        std::vector<float> host_a(kElementCount);
        std::vector<float> host_b(kElementCount);
        std::vector<float> host_c(kElementCount, 0.0F);
        for (int i = 0; i < kElementCount; ++i) {
            host_a[i] = static_cast<float>(i) * 0.5F;
            host_b[i] = static_cast<float>(i) * 0.25F;
        }

        check(cuMemAlloc(&resources.device_a, bytes), "cuMemAlloc(a)");
        check(cuMemAlloc(&resources.device_b, bytes), "cuMemAlloc(b)");
        check(cuMemAlloc(&resources.device_c, bytes), "cuMemAlloc(c)");
        check(cuMemcpyHtoD(resources.device_a, host_a.data(), bytes), "cuMemcpyHtoD(a)");
        check(cuMemcpyHtoD(resources.device_b, host_b.data(), bytes), "cuMemcpyHtoD(b)");

        void* parameters[]{
            &resources.device_a,
            &resources.device_b,
            &resources.device_c,
            &kernel_count,
        };
        const auto blocks = static_cast<unsigned int>(
            (kElementCount + threads - 1) / threads
        );
        check(
            cuLaunchKernel(
                function,
                blocks,
                1,
                1,
                static_cast<unsigned int>(threads),
                1,
                1,
                0,
                nullptr,
                parameters,
                nullptr
            ),
            "cuLaunchKernel"
        );
        check(cuCtxSynchronize(), "cuCtxSynchronize");
        check(cuMemcpyDtoH(host_c.data(), resources.device_c, bytes), "cuMemcpyDtoH(c)");

        std::size_t mismatch_count = 0;
        for (int i = 0; i < kElementCount; ++i) {
            const float expected = host_a[i] + host_b[i];
            if (!std::isfinite(host_c[i]) || std::fabs(host_c[i] - expected) > 1e-5F) {
                if (mismatch_count == 0) {
                    std::cerr << "mismatch_index=" << i << '\n';
                    std::cerr << "mismatch_expected=" << expected << '\n';
                    std::cerr << "mismatch_actual=" << host_c[i] << '\n';
                }
                ++mismatch_count;
            }
        }

        resources.release();

        std::cout << "device=" << device_name << '\n';
        std::cout << "compute_capability=" << compute_major << '.' << compute_minor << '\n';
        std::cout << "elements=" << kElementCount << '\n';
        std::cout << "threads=" << threads << '\n';
        std::cout << "blocks=" << blocks << '\n';
        std::cout << "verification=" << (mismatch_count == 0 ? "passed" : "failed") << '\n';
        return mismatch_count == 0 ? EXIT_SUCCESS : EXIT_FAILURE;
    } catch (const std::exception& error) {
        std::cerr << "error=" << error.what() << '\n';
        return EXIT_FAILURE;
    }
}
