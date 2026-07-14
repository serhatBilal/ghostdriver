extern "C" __global__ void vector_add(const float* a, const float* b, float* c, int count) {
    const int index = static_cast<int>(blockIdx.x * blockDim.x + threadIdx.x);
    if (index < count) {
        c[index] = a[index] + b[index];
    }
}
