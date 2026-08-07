package studio.luna.linker;

final class NativeBridge {
    static {
        System.loadLibrary("luna_mic_rust");
    }

    private NativeBridge() {}

    static native void nativeInit(String filesDir);
    static native String nativeHandle(String request);
}
