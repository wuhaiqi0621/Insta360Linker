package studio.insta360.linker;

final class NativeBridge {
    static {
        System.loadLibrary("insta360_linker");
    }

    private NativeBridge() {}

    static native void nativeInit(String filesDir);
    static native String nativeHandle(String request);
    static native byte[] nativePollPreview(int timeoutMs);
}
