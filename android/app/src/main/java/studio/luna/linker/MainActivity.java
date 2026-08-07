package studio.luna.linker;

import android.Manifest;
import android.annotation.SuppressLint;
import android.app.Activity;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothManager;
import android.bluetooth.le.BluetoothLeScanner;
import android.bluetooth.le.ScanCallback;
import android.bluetooth.le.ScanResult;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.media.MediaMetadataRetriever;
import android.media.MediaScannerConnection;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.os.Handler;
import android.os.Looper;
import android.provider.OpenableColumns;
import android.util.Base64;
import android.view.Window;
import android.window.OnBackInvokedDispatcher;
import android.webkit.JavascriptInterface;
import android.webkit.MimeTypeMap;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import org.json.JSONObject;
import org.json.JSONArray;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

public final class MainActivity extends Activity {
    private static final int PICK_MEDIA_REQUEST = 2001;
    private static final int PICK_MOMENT_REQUEST = 2002;
    private static final int MAX_THUMBNAIL_SOURCE = 96 * 1024 * 1024;

    private final ExecutorService nativeExecutor = Executors.newFixedThreadPool(3);
    private WebView webView;
    private JSONObject pendingPickerRequest;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        Window window = getWindow();
        window.setStatusBarColor(0xff121216);
        window.setNavigationBarColor(0xff121216);

        copyAssetTree("runtime_assets", new File(getFilesDir(), "assets"));
        NativeBridge.nativeInit(getFilesDir().getAbsolutePath());
        requestBluetoothPermissions();

        webView = new WebView(this);
        webView.setBackgroundColor(0xff121216);
        WebSettings settings = webView.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setAllowFileAccess(true);
        settings.setAllowContentAccess(true);
        settings.setAllowFileAccessFromFileURLs(true);
        settings.setAllowUniversalAccessFromFileURLs(true);
        settings.setMixedContentMode(WebSettings.MIXED_CONTENT_ALWAYS_ALLOW);
        settings.setMediaPlaybackRequiresUserGesture(false);
        settings.setBuiltInZoomControls(false);
        settings.setDisplayZoomControls(false);
        settings.setLoadWithOverviewMode(true);
        settings.setUseWideViewPort(true);

        webView.addJavascriptInterface(new IpcBridge(), "ipc");
        webView.setWebChromeClient(new WebChromeClient());
        webView.setWebViewClient(new WebViewClient() {
            @Override
            public void onPageFinished(WebView view, String url) {
                String script = "document.documentElement.classList.add('android-host','no-native-mica');"
                    + "const v=document.getElementById('virtualCameraControl');if(v)v.style.display='none';"
                    + "const p=document.getElementById('togglePreview');if(p)p.style.display='none';"
                    + "const s=document.createElement('style');s.textContent='.android-host .camera-console{grid-template-columns:minmax(0,1fr)!important}.android-host .live-surface{display:none!important}';document.head.appendChild(s);"
                    + "const b=document.querySelector('.brand-copy span');if(b)b.textContent='移动影像工作台';";
                view.evaluateJavascript(script, null);
            }

            @Override
            public boolean shouldOverrideUrlLoading(WebView view, String url) {
                if (url.startsWith("http://192.168.42.1/")) {
                    return false;
                }
                startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse(url)));
                return true;
            }
        });
        setContentView(webView);
        if (Build.VERSION.SDK_INT >= 33) {
            getOnBackInvokedDispatcher().registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
                this::handleBackNavigation
            );
        }
        webView.loadUrl("file:///android_asset/web/index.html");
    }

    @Override
    @SuppressLint("GestureBackNavigation")
    public void onBackPressed() {
        handleBackNavigation();
    }

    private void handleBackNavigation() {
        if (webView != null && webView.canGoBack()) {
            webView.goBack();
        } else {
            finish();
        }
    }

    @Override
    protected void onDestroy() {
        nativeExecutor.shutdownNow();
        if (webView != null) {
            webView.destroy();
        }
        super.onDestroy();
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        JSONObject request = pendingPickerRequest;
        pendingPickerRequest = null;
        if (request == null) {
            return;
        }
        if (resultCode != RESULT_OK || data == null || data.getData() == null) {
            sendSuccess(request, new JSONObject());
            return;
        }
        Uri uri = data.getData();
        nativeExecutor.submit(() -> {
            try {
                File imported = importDocument(uri);
                JSONObject result = new JSONObject();
                result.put("path", imported.getAbsolutePath());
                sendSuccess(request, result);
            } catch (Exception error) {
                sendError(request, "无法读取所选文件：" + error.getMessage());
            }
        });
    }

    private final class IpcBridge {
        @JavascriptInterface
        public void postMessage(String requestText) {
            try {
                JSONObject request = new JSONObject(requestText);
                String command = request.optString("command");
                switch (command) {
                    case "pick_media_file":
                        launchPicker(request, PICK_MEDIA_REQUEST, "image/*", "video/*");
                        return;
                    case "pick_moment_image":
                        launchPicker(request, PICK_MOMENT_REQUEST, "image/*");
                        return;
                    case "pick_watermark_output":
                        File pictures = new File(
                            getExternalFilesDir(Environment.DIRECTORY_PICTURES),
                            "Insta360Linker"
                        );
                        pictures.mkdirs();
                        JSONObject output = new JSONObject();
                        output.put("path", new File(pictures, "Luna_watermarked.jpg").getAbsolutePath());
                        sendSuccess(request, output);
                        return;
                    case "pick_download_dir":
                        File downloads = new File(
                            getExternalFilesDir(Environment.DIRECTORY_DOWNLOADS),
                            "Insta360Linker"
                        );
                        downloads.mkdirs();
                        JSONObject directory = new JSONObject();
                        directory.put("path", downloads.getAbsolutePath());
                        sendSuccess(request, directory);
                        return;
                    case "media_thumbnail":
                        nativeExecutor.submit(() -> createThumbnailResponse(request));
                        return;
                    case "scan_ble":
                        startBleScan(request);
                        return;
                    default:
                        nativeExecutor.submit(() -> {
                            String response = NativeBridge.nativeHandle(request.toString());
                            deliverResponse(response);
                            scanExportedFile(response);
                        });
                }
            } catch (Exception error) {
                deliverResponse(errorResponse(0, "parse", error.getMessage()).toString());
            }
        }
    }

    private void startBleScan(JSONObject request) {
        runOnUiThread(() -> {
            if (Build.VERSION.SDK_INT >= 31
                && (checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) != PackageManager.PERMISSION_GRANTED
                    || checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) != PackageManager.PERMISSION_GRANTED)) {
                sendError(request, "请允许附近设备权限后再扫描 Mic Pro");
                return;
            }
            if (Build.VERSION.SDK_INT <= 30
                && checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
                sendError(request, "请允许定位权限后再扫描蓝牙设备");
                return;
            }
            BluetoothManager manager = getSystemService(BluetoothManager.class);
            BluetoothAdapter adapter = manager == null ? null : manager.getAdapter();
            if (adapter == null || !adapter.isEnabled()) {
                sendError(request, "请先开启蓝牙");
                return;
            }
            BluetoothLeScanner scanner = adapter.getBluetoothLeScanner();
            if (scanner == null) {
                sendError(request, "当前设备无法启动蓝牙扫描");
                return;
            }

            Map<String, String> devices = new LinkedHashMap<>();
            AtomicBoolean completed = new AtomicBoolean(false);
            ScanCallback callback = new ScanCallback() {
                @Override
                public void onScanResult(int callbackType, ScanResult result) {
                    String address = result.getDevice().getAddress();
                    String name = null;
                    if (Build.VERSION.SDK_INT < 31
                        || checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) == PackageManager.PERMISSION_GRANTED) {
                        name = result.getDevice().getName();
                    }
                    if (name == null && result.getScanRecord() != null) {
                        name = result.getScanRecord().getDeviceName();
                    }
                    devices.put(address, name == null || name.trim().isEmpty() ? "蓝牙设备" : name);
                }

                @Override
                public void onScanFailed(int errorCode) {
                    if (completed.compareAndSet(false, true)) {
                        sendError(request, "蓝牙扫描失败：" + errorCode);
                    }
                }
            };
            scanner.startScan(callback);
            new Handler(Looper.getMainLooper()).postDelayed(() -> {
                try {
                    scanner.stopScan(callback);
                    if (!completed.compareAndSet(false, true)) {
                        return;
                    }
                    JSONArray result = new JSONArray();
                    for (Map.Entry<String, String> device : devices.entrySet()) {
                        String normalized = device.getValue().toLowerCase(Locale.ROOT);
                        if (!normalized.contains("mic") && !normalized.contains("insta360")) {
                            continue;
                        }
                        JSONObject item = new JSONObject();
                        item.put("name", device.getValue());
                        item.put("address", device.getKey());
                        result.put(item);
                    }
                    sendSuccess(request, result);
                } catch (Exception error) {
                    sendError(request, "无法整理蓝牙扫描结果：" + error.getMessage());
                }
            }, 5000);
        });
    }

    private void launchPicker(JSONObject request, int code, String... mimeTypes) {
        runOnUiThread(() -> {
            pendingPickerRequest = request;
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
            intent.addCategory(Intent.CATEGORY_OPENABLE);
            intent.setType(mimeTypes.length == 1 ? mimeTypes[0] : "*/*");
            if (mimeTypes.length > 1) {
                intent.putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes);
            }
            startActivityForResult(intent, code);
        });
    }

    private void createThumbnailResponse(JSONObject request) {
        try {
            JSONObject payload = request.getJSONObject("payload");
            String url = payload.getString("url");
            String mediaType = payload.optString("media_type");
            byte[] jpeg = "video".equals(mediaType)
                ? videoThumbnail(url)
                : imageThumbnail(url);
            JSONObject result = new JSONObject();
            result.put("url", url);
            result.put("mime", "image/jpeg");
            result.put("data", Base64.encodeToString(jpeg, Base64.NO_WRAP));
            sendSuccess(request, result);
        } catch (Exception error) {
            sendError(request, "无法生成预览图：" + error.getMessage());
        }
    }

    private byte[] imageThumbnail(String sourceUrl) throws Exception {
        HttpURLConnection connection = (HttpURLConnection) new URL(sourceUrl).openConnection();
        connection.setConnectTimeout(6000);
        connection.setReadTimeout(25000);
        connection.setRequestProperty("User-Agent", "Insta360 Linker Android/0.1");
        connection.setRequestProperty("Accept-Encoding", "identity");
        connection.connect();
        if (connection.getResponseCode() >= 400) {
            throw new IOException("HTTP " + connection.getResponseCode());
        }
        byte[] source;
        try (InputStream input = connection.getInputStream()) {
            source = readLimited(input, MAX_THUMBNAIL_SOURCE);
        } finally {
            connection.disconnect();
        }
        BitmapFactory.Options bounds = new BitmapFactory.Options();
        bounds.inJustDecodeBounds = true;
        BitmapFactory.decodeByteArray(source, 0, source.length, bounds);
        int sample = 1;
        while (bounds.outWidth / sample > 960 || bounds.outHeight / sample > 640) {
            sample *= 2;
        }
        BitmapFactory.Options options = new BitmapFactory.Options();
        options.inSampleSize = Math.max(1, sample);
        Bitmap decoded = BitmapFactory.decodeByteArray(source, 0, source.length, options);
        if (decoded == null) {
            throw new IOException("图片解码失败");
        }
        return compressThumbnail(decoded);
    }

    private byte[] videoThumbnail(String sourceUrl) throws Exception {
        MediaMetadataRetriever retriever = new MediaMetadataRetriever();
        try {
            Map<String, String> headers = new HashMap<>();
            headers.put("User-Agent", "Insta360 Linker Android/0.1");
            retriever.setDataSource(sourceUrl, headers);
            Bitmap frame = retriever.getFrameAtTime(200_000, MediaMetadataRetriever.OPTION_CLOSEST_SYNC);
            if (frame == null) {
                throw new IOException("视频没有可用首帧");
            }
            return compressThumbnail(frame);
        } finally {
            retriever.release();
        }
    }

    private byte[] compressThumbnail(Bitmap source) {
        int width = source.getWidth();
        int height = source.getHeight();
        float scale = Math.min(1f, Math.min(480f / width, 320f / height));
        Bitmap thumbnail = source;
        if (scale < 1f) {
            thumbnail = Bitmap.createScaledBitmap(
                source,
                Math.max(1, Math.round(width * scale)),
                Math.max(1, Math.round(height * scale)),
                true
            );
        }
        ByteArrayOutputStream output = new ByteArrayOutputStream(64 * 1024);
        thumbnail.compress(Bitmap.CompressFormat.JPEG, 78, output);
        if (thumbnail != source) {
            thumbnail.recycle();
        }
        source.recycle();
        return output.toByteArray();
    }

    private byte[] readLimited(InputStream input, int limit) throws IOException {
        ByteArrayOutputStream output = new ByteArrayOutputStream(256 * 1024);
        byte[] buffer = new byte[64 * 1024];
        int total = 0;
        int count;
        while ((count = input.read(buffer)) >= 0) {
            total += count;
            if (total > limit) {
                throw new IOException("素材过大");
            }
            output.write(buffer, 0, count);
        }
        return output.toByteArray();
    }

    private File importDocument(Uri uri) throws Exception {
        String name = queryDisplayName(uri);
        File directory = new File(getCacheDir(), "imports");
        directory.mkdirs();
        File output = new File(directory, System.currentTimeMillis() + "-" + safeName(name));
        try (InputStream input = getContentResolver().openInputStream(uri);
             FileOutputStream stream = new FileOutputStream(output)) {
            if (input == null) {
                throw new IOException("无法打开文件");
            }
            byte[] buffer = new byte[64 * 1024];
            int count;
            while ((count = input.read(buffer)) >= 0) {
                stream.write(buffer, 0, count);
            }
        }
        return output;
    }

    private String queryDisplayName(Uri uri) {
        try (android.database.Cursor cursor = getContentResolver().query(
            uri,
            new String[]{OpenableColumns.DISPLAY_NAME},
            null,
            null,
            null
        )) {
            if (cursor != null && cursor.moveToFirst()) {
                return cursor.getString(0);
            }
        } catch (Exception ignored) {
        }
        String extension = MimeTypeMap.getSingleton().getExtensionFromMimeType(
            getContentResolver().getType(uri)
        );
        return "media" + (extension == null ? "" : "." + extension);
    }

    private String safeName(String value) {
        String safe = value == null ? "media" : value.replaceAll("[\\\\/:*?\"<>|]", "_");
        return safe.trim().isEmpty() ? "media" : safe;
    }

    private void sendSuccess(JSONObject request, Object data) {
        try {
            JSONObject response = new JSONObject();
            response.put("id", request.optLong("id"));
            response.put("command", request.optString("command"));
            response.put("ok", true);
            response.put("data", data);
            response.put("error", JSONObject.NULL);
            deliverResponse(response.toString());
        } catch (Exception error) {
            sendError(request, error.getMessage());
        }
    }

    private void sendError(JSONObject request, String message) {
        deliverResponse(errorResponse(
            request.optLong("id"),
            request.optString("command"),
            message
        ).toString());
    }

    private JSONObject errorResponse(long id, String command, String message) {
        JSONObject response = new JSONObject();
        try {
            response.put("id", id);
            response.put("command", command);
            response.put("ok", false);
            response.put("data", JSONObject.NULL);
            response.put("error", message == null ? "操作失败" : message);
        } catch (Exception ignored) {
        }
        return response;
    }

    private void deliverResponse(String response) {
        runOnUiThread(() -> webView.evaluateJavascript(
            "window.LunaBridge&&window.LunaBridge.receive(" + response + ");",
            null
        ));
    }

    private void scanExportedFile(String responseText) {
        try {
            JSONObject response = new JSONObject(responseText);
            if (!response.optBoolean("ok") || !"watermark".equals(response.optString("command"))) {
                return;
            }
            String path = response.getJSONObject("data").optString("path");
            if (!path.isEmpty()) {
                MediaScannerConnection.scanFile(this, new String[]{path}, null, null);
            }
        } catch (Exception ignored) {
        }
    }

    private void requestBluetoothPermissions() {
        if (Build.VERSION.SDK_INT <= 30) {
            if (checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
                requestPermissions(new String[]{Manifest.permission.ACCESS_FINE_LOCATION}, 3001);
            }
            return;
        }
        List<String> missing = new ArrayList<>();
        if (checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) != PackageManager.PERMISSION_GRANTED) {
            missing.add(Manifest.permission.BLUETOOTH_SCAN);
        }
        if (checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) != PackageManager.PERMISSION_GRANTED) {
            missing.add(Manifest.permission.BLUETOOTH_CONNECT);
        }
        if (!missing.isEmpty()) {
            requestPermissions(missing.toArray(new String[0]), 3001);
        }
    }

    private void copyAssetTree(String source, File destination) {
        try {
            String[] children = getAssets().list(source);
            if (children == null || children.length == 0) {
                File parent = destination.getParentFile();
                if (parent != null) {
                    parent.mkdirs();
                }
                try (InputStream input = getAssets().open(source);
                     FileOutputStream output = new FileOutputStream(destination)) {
                    byte[] buffer = new byte[32 * 1024];
                    int count;
                    while ((count = input.read(buffer)) >= 0) {
                        output.write(buffer, 0, count);
                    }
                }
                return;
            }
            destination.mkdirs();
            for (String child : children) {
                copyAssetTree(source + "/" + child, new File(destination, child));
            }
        } catch (IOException error) {
            throw new IllegalStateException("无法准备运行资源：" + source, error);
        }
    }
}
