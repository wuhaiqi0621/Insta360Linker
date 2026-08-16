package studio.insta360.linker;

import android.Manifest;
import android.annotation.SuppressLint;
import android.app.Activity;
import android.bluetooth.BluetoothAdapter;
import android.bluetooth.BluetoothManager;
import android.bluetooth.le.BluetoothLeScanner;
import android.bluetooth.le.ScanCallback;
import android.bluetooth.le.ScanResult;
import android.content.ContentResolver;
import android.content.ContentValues;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.content.res.Configuration;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.ImageFormat;
import android.graphics.Rect;
import android.graphics.YuvImage;
import android.media.Image;
import android.media.ImageReader;
import android.media.MediaCodec;
import android.media.MediaFormat;
import android.media.MediaMetadataRetriever;
import android.media.MediaScannerConnection;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Environment;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.os.SystemClock;
import android.provider.OpenableColumns;
import android.provider.MediaStore;
import android.util.Base64;
import android.view.View;
import android.view.Window;
import android.window.OnBackInvokedDispatcher;
import android.webkit.JavascriptInterface;
import android.webkit.MimeTypeMap;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import androidx.media3.common.MediaItem;
import androidx.media3.common.MimeTypes;
import androidx.media3.common.util.UnstableApi;
import androidx.media3.effect.BitmapOverlay;
import androidx.media3.effect.OverlayEffect;
import androidx.media3.effect.StaticOverlaySettings;
import androidx.media3.transformer.Composition;
import androidx.media3.transformer.EditedMediaItem;
import androidx.media3.transformer.Effects;
import androidx.media3.transformer.ExportException;
import androidx.media3.transformer.ExportResult;
import androidx.media3.transformer.ProgressHolder;
import androidx.media3.transformer.Transformer;

import org.json.JSONObject;
import org.json.JSONArray;

import java.io.ByteArrayOutputStream;
import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.FilterInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.ByteBuffer;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;

@UnstableApi
public final class MainActivity extends Activity {
    private static final int PICK_MEDIA_REQUEST = 2001;
    private static final int PICK_MOMENT_REQUEST = 2002;
    private static final int MAX_THUMBNAIL_SOURCE = 96 * 1024 * 1024;
    private static final long PREVIEW_FRAME_INTERVAL_MS = 120;

    private final ExecutorService nativeExecutor = Executors.newFixedThreadPool(3);
    private final ExecutorService transferExecutor = Executors.newSingleThreadExecutor();
    private final AtomicBoolean previewPumpRunning = new AtomicBoolean(false);
    private final AtomicBoolean taskEventPumpRunning = new AtomicBoolean(false);
    private final Object transferTaskLock = new Object();
    private final LinkedHashMap<Long, TransferTask> transferTasks = new LinkedHashMap<>();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private WebView webView;
    private JSONObject pendingPickerRequest;
    private Thread previewPumpThread;
    private Thread taskEventPumpThread;
    private Transformer activeVideoTransformer;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        int chromeColor = configureSystemAppearance();

        copyAssetTree("runtime_assets", new File(getFilesDir(), "assets"));
        NativeBridge.nativeInit(getFilesDir().getAbsolutePath());
        startTaskEventPump();
        requestBluetoothPermissions();

        webView = new WebView(this);
        webView.setBackgroundColor(chromeColor);
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
                String script = "document.documentElement.classList.remove('native-liquid-glass','native-mica','no-native-surface','macos-host','windows-host');document.documentElement.classList.add('android-host');"
                    + "const v=document.getElementById('virtualCameraControl');if(v)v.style.display='none';"
                    + "const d=document.getElementById('batchDownload');if(d)d.textContent='保存所选';"
                    + "const w=document.getElementById('exportWatermark');if(w)w.textContent='保存到相册';"
                    + "const b=document.querySelector('.brand-copy span');if(b)b.textContent='移动影像工作台';";
                view.evaluateJavascript(script, null);
                emitAllTransferTasks();
            }

            @Override
            public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest request) {
                Uri uri = request.getUrl();
                if ("appassets.androidplatform.net".equalsIgnoreCase(uri.getHost())
                    && uri.getPath() != null
                    && uri.getPath().startsWith("/camera-media/")) {
                    try {
                        String encoded = uri.getPath().substring("/camera-media/".length());
                        return openCameraMediaResponse(
                            decodeHexUrl(encoded),
                            request.getMethod(),
                            request.getRequestHeaders()
                        );
                    } catch (Exception error) {
                        return textResourceResponse(502, "相机媒体代理失败：" + error.getMessage());
                    }
                }
                return super.shouldInterceptRequest(view, request);
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
    protected void onDestroy() {
        stopPreviewPump();
        stopTaskEventPump();
        if (activeVideoTransformer != null) {
            activeVideoTransformer.cancel();
            activeVideoTransformer = null;
        }
        transferExecutor.shutdownNow();
        nativeExecutor.shutdownNow();
        if (webView != null) {
            webView.destroy();
        }
        super.onDestroy();
    }

    private int configureSystemAppearance() {
        Window window = getWindow();
        boolean darkAppearance = (getResources().getConfiguration().uiMode
            & Configuration.UI_MODE_NIGHT_MASK) == Configuration.UI_MODE_NIGHT_YES;
        int chromeColor = darkAppearance ? 0xff151b1d : 0xffedf1f2;
        window.setStatusBarColor(chromeColor);
        window.setNavigationBarColor(chromeColor);
        int systemUi = 0;
        if (!darkAppearance && Build.VERSION.SDK_INT >= 23) {
            systemUi |= View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
        }
        if (!darkAppearance && Build.VERSION.SDK_INT >= 26) {
            systemUi |= View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
        }
        window.getDecorView().setSystemUiVisibility(systemUi);
        return chromeColor;
    }

    @Override
    public void onConfigurationChanged(Configuration newConfig) {
        super.onConfigurationChanged(newConfig);
        int chromeColor = configureSystemAppearance();
        if (webView != null) {
            webView.setBackgroundColor(chromeColor);
        }
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
                        JSONObject payload = request.optJSONObject("payload");
                        String input = payload == null ? "" : payload.optString("input");
                        File watermarkExports = new File(getCacheDir(), "exports/watermark");
                        watermarkExports.mkdirs();
                        JSONObject output = new JSONObject();
                        output.put(
                            "path",
                            new File(
                                watermarkExports,
                                "Insta360Linker_watermarked_" + System.currentTimeMillis() + watermarkOutputExtension(input)
                            ).getAbsolutePath()
                        );
                        sendSuccess(request, output);
                        return;
                    case "pick_download_dir":
                        File downloads = new File(
                            getCacheDir(),
                            "exports/download_" + System.currentTimeMillis()
                        );
                        downloads.mkdirs();
                        JSONObject directory = new JSONObject();
                        directory.put("path", downloads.getAbsolutePath());
                        sendSuccess(request, directory);
                        return;
                    case "media_thumbnail":
                        nativeExecutor.submit(() -> createThumbnailResponse(request));
                        return;
                    case "download_batch":
                    case "prepare_watermark_media":
                    case "watermark":
                        enqueueTransferTask(request);
                        return;
                    case "scan_ble":
                        startBleScan(request);
                        return;
                    default:
                        nativeExecutor.submit(() -> {
                            String response = NativeBridge.nativeHandle(request.toString());
                            updatePreviewPump(command, response);
                            deliverResponse(response);
                        });
                }
            } catch (Exception error) {
                deliverResponse(errorResponse(0, "parse", error.getMessage()).toString());
            }
        }
    }

    private void enqueueTransferTask(JSONObject request) {
        long id = request.optLong("id");
        String command = request.optString("command");
        TransferTask task = new TransferTask(id, command, transferTaskTitle(command, request));
        synchronized (transferTaskLock) {
            trimTransferTaskHistory();
            transferTasks.put(id, task);
        }
        emitTransferTask(id);
        transferExecutor.submit(() -> executeTransferTask(request, task));
    }

    private String transferTaskTitle(String command, JSONObject request) {
        JSONObject payload = request.optJSONObject("payload");
        if ("download_batch".equals(command)) {
            int count = payload == null || payload.optJSONArray("files") == null
                ? 0
                : payload.optJSONArray("files").length();
            return count > 1 ? "保存 " + count + " 个相机素材" : "保存相机素材";
        }
        if ("prepare_watermark_media".equals(command)) {
            return "载入水印原片";
        }
        String input = payload == null ? "" : payload.optString("input");
        return isVideoFile(input) ? "导出视频水印" : "导出照片水印";
    }

    private void executeTransferTask(JSONObject request, TransferTask task) {
        updateTransferTask(task.id, "running", "正在准备", 1, null, null, 0, null, null);
        String response;
        try {
            JSONObject payload = request.optJSONObject("payload");
            if ("watermark".equals(task.command)
                && payload != null
                && isVideoFile(payload.optString("input"))) {
                response = exportAndroidVideoWatermark(request, task);
            } else {
                if ("watermark".equals(task.command)) {
                    updateTransferTask(task.id, "running", "正在渲染照片水印", 12, null, null, 0, null, null);
                }
                response = NativeBridge.nativeHandle(request.toString());
            }

            JSONObject parsed = new JSONObject(response);
            if (parsed.optBoolean("ok")
                && ("watermark".equals(task.command) || "download_batch".equals(task.command))) {
                response = publishExportsToGallery(response, task);
                parsed = new JSONObject(response);
            }

            if (parsed.optBoolean("ok")) {
                JSONObject data = parsed.optJSONObject("data");
                String message = data == null ? "任务已完成" : data.optString("message", "任务已完成");
                updateTransferTask(task.id, "completed", message, 100, null, null, 0, 0L, message);
            } else {
                String error = parsed.optString("error", "任务失败");
                updateTransferTask(task.id, "failed", "任务失败", null, null, null, 0, null, error);
            }
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            response = errorResponse(task.id, task.command, "任务已中断").toString();
            updateTransferTask(task.id, "failed", "任务已中断", null, null, null, 0, null, "任务已中断");
        } catch (Exception error) {
            String message = error.getMessage() == null ? "任务失败" : error.getMessage();
            response = errorResponse(task.id, task.command, message).toString();
            updateTransferTask(task.id, "failed", "任务失败", null, null, null, 0, null, message);
        }
        deliverResponse(response);
    }

    private void startTaskEventPump() {
        if (!taskEventPumpRunning.compareAndSet(false, true)) {
            return;
        }
        taskEventPumpThread = new Thread(() -> {
            while (taskEventPumpRunning.get() && !Thread.currentThread().isInterrupted()) {
                String event = NativeBridge.nativePollTaskEvent(500);
                if (event == null || event.isEmpty()) {
                    continue;
                }
                try {
                    applyNativeTaskEvent(new JSONObject(event));
                } catch (Exception ignored) {
                }
            }
        }, "Insta360Linker-task-events");
        taskEventPumpThread.start();
    }

    private void stopTaskEventPump() {
        taskEventPumpRunning.set(false);
        Thread thread = taskEventPumpThread;
        taskEventPumpThread = null;
        if (thread != null) {
            thread.interrupt();
        }
    }

    private void applyNativeTaskEvent(JSONObject event) {
        long id = event.optLong("id");
        Integer progress = event.has("progress") && !event.isNull("progress")
            ? event.optInt("progress")
            : null;
        Long completedBytes = event.has("completed_bytes") && !event.isNull("completed_bytes")
            ? event.optLong("completed_bytes")
            : null;
        Long totalBytes = event.has("total_bytes") && !event.isNull("total_bytes")
            ? event.optLong("total_bytes")
            : null;
        Long etaSeconds = event.has("eta_seconds") && !event.isNull("eta_seconds")
            ? event.optLong("eta_seconds")
            : null;
        String itemName = event.optString("item_name", "");
        int itemIndex = event.optInt("item_index", 0);
        int itemCount = event.optInt("item_count", 0);
        String detail = itemName;
        if (itemCount > 1) {
            detail = itemIndex + "/" + itemCount + (itemName.isEmpty() ? "" : " · " + itemName);
        }
        updateTransferTask(
            id,
            "running",
            event.optString("phase", "正在处理"),
            progress,
            completedBytes,
            totalBytes,
            event.optLong("speed_bps", 0),
            etaSeconds,
            detail
        );
    }

    private void updateTransferTask(
        long id,
        String state,
        String phase,
        Integer progress,
        Long completedBytes,
        Long totalBytes,
        long speedBps,
        Long etaSeconds,
        String detail
    ) {
        synchronized (transferTaskLock) {
            TransferTask task = transferTasks.get(id);
            if (task == null) {
                return;
            }
            task.state = state;
            task.phase = phase;
            if (progress != null) {
                task.progress = Math.max(0, Math.min(100, progress));
            }
            if (completedBytes != null) {
                task.completedBytes = completedBytes;
            }
            if (totalBytes != null) {
                task.totalBytes = totalBytes;
            }
            task.speedBps = speedBps;
            task.etaSeconds = etaSeconds;
            if (detail != null) {
                task.detail = detail;
            }
            task.updatedAt = System.currentTimeMillis();
        }
        emitTransferTask(id);
    }

    private void emitTransferTask(long id) {
        JSONObject snapshot;
        synchronized (transferTaskLock) {
            TransferTask task = transferTasks.get(id);
            if (task == null) {
                return;
            }
            snapshot = task.toJson();
        }
        if (webView == null) {
            return;
        }
        String script = "window.Insta360LinkerBridge&&window.Insta360LinkerBridge.taskProgress("
            + snapshot + ");";
        runOnUiThread(() -> webView.evaluateJavascript(script, null));
    }

    private void emitAllTransferTasks() {
        List<Long> ids;
        synchronized (transferTaskLock) {
            ids = new ArrayList<>(transferTasks.keySet());
        }
        for (Long id : ids) {
            emitTransferTask(id);
        }
    }

    private void trimTransferTaskHistory() {
        while (transferTasks.size() >= 24) {
            Long removable = null;
            for (Map.Entry<Long, TransferTask> entry : transferTasks.entrySet()) {
                if (entry.getValue().isTerminal()) {
                    removable = entry.getKey();
                    break;
                }
            }
            if (removable == null) {
                return;
            }
            transferTasks.remove(removable);
        }
    }

    private static final class TransferTask {
        final long id;
        final String command;
        final String title;
        String state = "queued";
        String phase = "等待前面的任务";
        String detail = "";
        int progress = 0;
        long completedBytes = 0;
        long totalBytes = 0;
        long speedBps = 0;
        Long etaSeconds;
        final long createdAt = System.currentTimeMillis();
        long updatedAt = createdAt;

        TransferTask(long id, String command, String title) {
            this.id = id;
            this.command = command;
            this.title = title;
        }

        boolean isTerminal() {
            return "completed".equals(state) || "failed".equals(state);
        }

        JSONObject toJson() {
            JSONObject value = new JSONObject();
            try {
                value.put("id", id);
                value.put("command", command);
                value.put("title", title);
                value.put("state", state);
                value.put("phase", phase);
                value.put("detail", detail);
                value.put("progress", progress);
                value.put("completed_bytes", completedBytes);
                value.put("total_bytes", totalBytes > 0 ? totalBytes : JSONObject.NULL);
                value.put("speed_bps", speedBps);
                value.put("eta_seconds", etaSeconds == null ? JSONObject.NULL : etaSeconds);
                value.put("created_at", createdAt);
                value.put("updated_at", updatedAt);
            } catch (Exception ignored) {
            }
            return value;
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

    private WebResourceResponse openCameraMediaResponse(
        String sourceUrl,
        String method,
        Map<String, String> requestHeaders
    ) throws IOException {
        HttpURLConnection connection = (HttpURLConnection) new URL(sourceUrl).openConnection();
        connection.setConnectTimeout(8000);
        connection.setReadTimeout(30 * 60 * 1000);
        connection.setRequestMethod("HEAD".equalsIgnoreCase(method) ? "HEAD" : "GET");
        connection.setRequestProperty("User-Agent", "Insta360Linker Android/0.2");
        connection.setRequestProperty("Accept", "*/*");
        connection.setRequestProperty("Accept-Encoding", "identity");
        for (Map.Entry<String, String> header : requestHeaders.entrySet()) {
            if ("range".equalsIgnoreCase(header.getKey())
                || "if-range".equalsIgnoreCase(header.getKey())) {
                connection.setRequestProperty(header.getKey(), header.getValue());
            }
        }
        connection.connect();

        int status = connection.getResponseCode();
        String contentType = connection.getContentType();
        String mime = contentType == null
            ? mediaMimeType(sourceUrl, isVideoFile(sourceUrl))
            : contentType.split(";", 2)[0].trim();
        Map<String, String> responseHeaders = new HashMap<>();
        copyResponseHeader(connection, responseHeaders, "Accept-Ranges");
        copyResponseHeader(connection, responseHeaders, "Content-Length");
        copyResponseHeader(connection, responseHeaders, "Content-Range");
        copyResponseHeader(connection, responseHeaders, "ETag");
        copyResponseHeader(connection, responseHeaders, "Last-Modified");
        responseHeaders.put("Access-Control-Allow-Origin", "*");
        responseHeaders.put("Cache-Control", "no-store");

        InputStream source;
        if ("HEAD".equalsIgnoreCase(method)) {
            source = new ByteArrayInputStream(new byte[0]);
        } else {
            source = status >= 400 ? connection.getErrorStream() : connection.getInputStream();
            if (source == null) {
                source = new ByteArrayInputStream(new byte[0]);
            }
        }
        final InputStream responseStream = source;
        InputStream closingStream = new FilterInputStream(responseStream) {
            @Override
            public void close() throws IOException {
                try {
                    super.close();
                } finally {
                    connection.disconnect();
                }
            }
        };
        return new WebResourceResponse(
            mime,
            null,
            status,
            httpReason(status),
            responseHeaders,
            closingStream
        );
    }

    private void copyResponseHeader(
        HttpURLConnection connection,
        Map<String, String> destination,
        String name
    ) {
        String value = connection.getHeaderField(name);
        if (value != null && !value.isEmpty()) {
            destination.put(name, value);
        }
    }

    private WebResourceResponse textResourceResponse(int status, String message) {
        byte[] data = message.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        return new WebResourceResponse(
            "text/plain",
            "utf-8",
            status,
            httpReason(status),
            new HashMap<>(),
            new ByteArrayInputStream(data)
        );
    }

    private String decodeHexUrl(String encoded) throws IOException {
        if (encoded.isEmpty() || (encoded.length() & 1) != 0) {
            throw new IOException("媒体地址编码无效");
        }
        byte[] bytes = new byte[encoded.length() / 2];
        try {
            for (int index = 0; index < bytes.length; index++) {
                bytes[index] = (byte) Integer.parseInt(encoded.substring(index * 2, index * 2 + 2), 16);
            }
            return new String(bytes, java.nio.charset.StandardCharsets.UTF_8);
        } catch (NumberFormatException error) {
            throw new IOException("媒体地址编码无效", error);
        }
    }

    private String httpReason(int status) {
        switch (status) {
            case 200: return "OK";
            case 206: return "Partial Content";
            case 400: return "Bad Request";
            case 403: return "Forbidden";
            case 404: return "Not Found";
            case 416: return "Range Not Satisfiable";
            case 502: return "Bad Gateway";
            default: return status >= 400 ? "Camera Media Error" : "Camera Media";
        }
    }

    private void updatePreviewPump(String command, String responseText) {
        try {
            JSONObject response = new JSONObject(responseText);
            if (!response.optBoolean("ok")) {
                if ("camera_start_preview".equals(command)) {
                    stopPreviewPump();
                }
                return;
            }
            if ("camera_start_preview".equals(command)) {
                startPreviewPump();
            } else if ("camera_stop_preview".equals(command)
                || "disconnect_luna".equals(command)) {
                stopPreviewPump();
            }
        } catch (Exception error) {
            if ("camera_start_preview".equals(command)) {
                deliverPreviewError("无法启动实时预览：" + error.getMessage());
            }
        }
    }

    private void startPreviewPump() {
        if (!previewPumpRunning.compareAndSet(false, true)) {
            return;
        }
        previewPumpThread = new Thread(this::runPreviewPump, "Insta360Linker-preview");
        previewPumpThread.start();
    }

    private void stopPreviewPump() {
        previewPumpRunning.set(false);
        Thread thread = previewPumpThread;
        previewPumpThread = null;
        if (thread != null) {
            thread.interrupt();
        }
    }

    private void runPreviewPump() {
        HandlerThread imageThread = new HandlerThread("Insta360Linker-preview-images");
        ImageReader imageReader = null;
        MediaCodec decoder = null;
        byte[] activeCodecConfig = null;
        try {
            imageThread.start();
            imageReader = ImageReader.newInstance(1280, 720, ImageFormat.YUV_420_888, 3);
            final long[] lastFrameAt = {0L};
            AtomicLong decodedFrameAt = new AtomicLong(0L);
            imageReader.setOnImageAvailableListener(reader -> {
                try (Image image = reader.acquireLatestImage()) {
                    if (image == null || !previewPumpRunning.get()) {
                        return;
                    }
                    long now = SystemClock.elapsedRealtime();
                    decodedFrameAt.set(now);
                    if (now - lastFrameAt[0] < PREVIEW_FRAME_INTERVAL_MS) {
                        return;
                    }
                    lastFrameAt[0] = now;
                    deliverPreviewImage(imageToJpeg(image));
                } catch (Exception error) {
                    deliverPreviewError("实时画面转换失败：" + error.getMessage());
                }
            }, new Handler(imageThread.getLooper()));

            MediaCodec.BufferInfo outputInfo = new MediaCodec.BufferInfo();
            long lastPresentationTimeUs = 0L;
            long decoderStartedAt = 0L;
            int decoderFailures = 0;
            while (previewPumpRunning.get() && !Thread.currentThread().isInterrupted()) {
                if (decoder != null) {
                    long lastOutputAt = decodedFrameAt.get();
                    long outputReference = lastOutputAt > 0 ? lastOutputAt : decoderStartedAt;
                    if (outputReference > 0
                        && SystemClock.elapsedRealtime() - outputReference > 6_000) {
                        decoder = releasePreviewDecoder(decoder);
                        activeCodecConfig = null;
                        decoderStartedAt = 0L;
                        decodedFrameAt.set(0L);
                        decoderFailures++;
                        deliverPreviewRecovering("解码器暂无画面，正在等待下一个关键帧自动恢复");
                    }
                }
                byte[] chunk = NativeBridge.nativePollPreview(250);
                if (chunk == null || chunk.length == 0) {
                    if (decoder != null) {
                        drainDecoder(decoder, outputInfo);
                    }
                    continue;
                }

                HevcAccessUnit.Parsed accessUnit = HevcAccessUnit.parse(chunk);
                boolean codecConfigChanged = accessUnit.hasCompleteCodecConfig
                    && !Arrays.equals(activeCodecConfig, accessUnit.codecConfig);
                if (decoder == null || codecConfigChanged) {
                    if (!accessUnit.canStartDecoder()) {
                        continue;
                    }
                    decoder = releasePreviewDecoder(decoder);
                    try {
                        decoder = createPreviewDecoder(imageReader, accessUnit.codecConfig);
                        activeCodecConfig = accessUnit.codecConfig;
                        decoderStartedAt = SystemClock.elapsedRealtime();
                        decodedFrameAt.set(0L);
                        deliverPreviewRecovering("关键帧已同步，正在解码");
                    } catch (Exception error) {
                        decoder = releasePreviewDecoder(decoder);
                        activeCodecConfig = null;
                        decoderStartedAt = 0L;
                        decodedFrameAt.set(0L);
                        decoderFailures++;
                        deliverPreviewRecovering("解码器启动失败，正在等待下一个关键帧");
                        if (decoderFailures >= 3) {
                            deliverPreviewError("Android HEVC 解码器无法启动：" + error.getMessage());
                        }
                        continue;
                    }
                }

                if (accessUnit.sample.length == 0) {
                    continue;
                }

                try {
                    long presentationTimeUs = Math.max(
                        lastPresentationTimeUs + 1,
                        SystemClock.elapsedRealtimeNanos() / 1000L
                    );
                    queuePreviewSample(decoder, outputInfo, accessUnit, presentationTimeUs);
                    lastPresentationTimeUs = presentationTimeUs;
                    decoderFailures = 0;
                } catch (Exception error) {
                    decoder = releasePreviewDecoder(decoder);
                    activeCodecConfig = null;
                    decoderStartedAt = 0L;
                    decodedFrameAt.set(0L);
                    decoderFailures++;
                    deliverPreviewRecovering("画面中断，正在等待下一个关键帧自动恢复");
                    if (decoderFailures >= 3) {
                        deliverPreviewError("Android 实时预览解码失败：" + error.getMessage());
                    }
                }
            }
        } catch (Exception error) {
            if (previewPumpRunning.get()) {
                deliverPreviewError("Android 实时预览解码失败：" + error.getMessage());
            }
        } finally {
            previewPumpRunning.set(false);
            releasePreviewDecoder(decoder);
            if (imageReader != null) {
                imageReader.close();
            }
            imageThread.quitSafely();
        }
    }

    private MediaCodec createPreviewDecoder(ImageReader imageReader, byte[] codecConfig) throws IOException {
        MediaCodec decoder = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_VIDEO_HEVC);
        try {
            MediaFormat format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_HEVC, 1280, 720);
            format.setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, 2 * 1024 * 1024);
            format.setByteBuffer("csd-0", ByteBuffer.wrap(codecConfig));
            decoder.configure(format, imageReader.getSurface(), null, 0);
            decoder.start();
            return decoder;
        } catch (Exception error) {
            decoder.release();
            if (error instanceof IOException) {
                throw (IOException) error;
            }
            throw new IOException(error.getMessage(), error);
        }
    }

    private void queuePreviewSample(
        MediaCodec decoder,
        MediaCodec.BufferInfo outputInfo,
        HevcAccessUnit.Parsed accessUnit,
        long presentationTimeUs
    ) throws IOException {
        long deadline = SystemClock.elapsedRealtime() + 750;
        while (previewPumpRunning.get() && !Thread.currentThread().isInterrupted()) {
            int inputIndex = decoder.dequeueInputBuffer(20_000);
            if (inputIndex >= 0) {
                ByteBuffer input = decoder.getInputBuffer(inputIndex);
                if (input == null || input.capacity() < accessUnit.sample.length) {
                    decoder.queueInputBuffer(inputIndex, 0, 0, presentationTimeUs, 0);
                    throw new IOException("相机预览帧超过系统解码器容量");
                }
                input.clear();
                input.put(accessUnit.sample);
                decoder.queueInputBuffer(
                    inputIndex,
                    0,
                    accessUnit.sample.length,
                    presentationTimeUs,
                    accessUnit.keyFrame ? MediaCodec.BUFFER_FLAG_KEY_FRAME : 0
                );
                drainDecoder(decoder, outputInfo);
                return;
            }
            drainDecoder(decoder, outputInfo);
            if (SystemClock.elapsedRealtime() >= deadline) {
                throw new IOException("系统解码器输入缓冲区持续繁忙");
            }
        }
    }

    private MediaCodec releasePreviewDecoder(MediaCodec decoder) {
        if (decoder == null) {
            return null;
        }
        try {
            decoder.stop();
        } catch (Exception ignored) {
        }
        decoder.release();
        return null;
    }

    private void drainDecoder(MediaCodec decoder, MediaCodec.BufferInfo outputInfo) {
        while (true) {
            int outputIndex = decoder.dequeueOutputBuffer(outputInfo, 0);
            if (outputIndex >= 0) {
                decoder.releaseOutputBuffer(outputIndex, true);
                continue;
            }
            if (outputIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                continue;
            }
            break;
        }
    }

    private byte[] imageToJpeg(Image image) throws IOException {
        Rect crop = image.getCropRect();
        int width = crop.width();
        int height = crop.height();
        if ((width & 1) != 0) width--;
        if ((height & 1) != 0) height--;
        byte[] nv21 = new byte[width * height * 3 / 2];
        Image.Plane[] planes = image.getPlanes();
        copyYPlane(planes[0], crop.left, crop.top, width, height, nv21);
        copyChromaPlanes(planes[1], planes[2], crop.left, crop.top, width, height, nv21);
        ByteArrayOutputStream output = new ByteArrayOutputStream(160 * 1024);
        YuvImage yuv = new YuvImage(nv21, ImageFormat.NV21, width, height, null);
        if (!yuv.compressToJpeg(new Rect(0, 0, width, height), 76, output)) {
            throw new IOException("系统无法编码实时预览帧");
        }
        return output.toByteArray();
    }

    private void copyYPlane(
        Image.Plane plane,
        int cropLeft,
        int cropTop,
        int width,
        int height,
        byte[] output
    ) {
        ByteBuffer buffer = plane.getBuffer().duplicate();
        int base = buffer.position();
        int rowStride = plane.getRowStride();
        int pixelStride = plane.getPixelStride();
        int target = 0;
        for (int y = 0; y < height; y++) {
            int row = base + (cropTop + y) * rowStride + cropLeft * pixelStride;
            for (int x = 0; x < width; x++) {
                output[target++] = buffer.get(row + x * pixelStride);
            }
        }
    }

    private void copyChromaPlanes(
        Image.Plane uPlane,
        Image.Plane vPlane,
        int cropLeft,
        int cropTop,
        int width,
        int height,
        byte[] output
    ) {
        ByteBuffer u = uPlane.getBuffer().duplicate();
        ByteBuffer v = vPlane.getBuffer().duplicate();
        int uBase = u.position();
        int vBase = v.position();
        int target = width * height;
        for (int y = 0; y < height / 2; y++) {
            int sourceY = cropTop / 2 + y;
            int uRow = uBase + sourceY * uPlane.getRowStride();
            int vRow = vBase + sourceY * vPlane.getRowStride();
            for (int x = 0; x < width / 2; x++) {
                int sourceX = cropLeft / 2 + x;
                output[target++] = v.get(vRow + sourceX * vPlane.getPixelStride());
                output[target++] = u.get(uRow + sourceX * uPlane.getPixelStride());
            }
        }
    }

    private void deliverPreviewImage(byte[] jpeg) {
        String data = Base64.encodeToString(jpeg, Base64.NO_WRAP);
        runOnUiThread(() -> webView.evaluateJavascript(
            "window.Insta360LinkerBridge&&window.Insta360LinkerBridge.previewImage({data:"
                + JSONObject.quote(data) + "});",
            null
        ));
    }

    private void deliverPreviewError(String message) {
        runOnUiThread(() -> webView.evaluateJavascript(
            "window.Insta360LinkerBridge&&window.Insta360LinkerBridge.previewError("
                + JSONObject.quote(message) + ");",
            null
        ));
    }

    private void deliverPreviewRecovering(String message) {
        runOnUiThread(() -> webView.evaluateJavascript(
            "window.Insta360LinkerBridge&&window.Insta360LinkerBridge.previewRecovering("
                + JSONObject.quote(message) + ");",
            null
        ));
    }

    @androidx.media3.common.util.UnstableApi
    private String exportAndroidVideoWatermark(JSONObject request, TransferTask task) throws Exception {
        Bitmap sourceBitmap = null;
        try {
            updateTransferTask(task.id, "running", "正在准备视频水印", 4, null, null, 0, null, null);
            JSONObject payload = request.getJSONObject("payload");
            String input = payload.getString("input");
            String output = payload.getString("output");
            int[] dimensions = videoDimensions(input);

            JSONObject planPayload = new JSONObject();
            planPayload.put("width", dimensions[0]);
            planPayload.put("height", dimensions[1]);
            planPayload.put("style", payload.optString("style", "luna-ultra-cn"));
            planPayload.put("position", payload.optString("position", "bottom-center"));
            JSONObject planRequest = new JSONObject();
            planRequest.put("id", 0);
            planRequest.put("command", "watermark_video_plan");
            planRequest.put("payload", planPayload);
            JSONObject planResponse = new JSONObject(NativeBridge.nativeHandle(planRequest.toString()));
            if (!planResponse.optBoolean("ok")) {
                throw new IOException(planResponse.optString("error", "无法读取官方视频水印参数"));
            }
            JSONObject plan = planResponse.getJSONObject("data");
            byte[] watermarkBytes = Base64.decode(plan.getString("data"), Base64.DEFAULT);
            sourceBitmap = BitmapFactory.decodeByteArray(watermarkBytes, 0, watermarkBytes.length);
            if (sourceBitmap == null) {
                throw new IOException("官方视频水印资源无法解码");
            }

            int targetWidth = Math.max(1, Math.round(dimensions[0] * (float) plan.getDouble("width_ratio")));
            int targetHeight = Math.max(
                1,
                Math.round(sourceBitmap.getHeight() * targetWidth / (float) sourceBitmap.getWidth())
            );
            Bitmap overlayBitmap = Bitmap.createScaledBitmap(sourceBitmap, targetWidth, targetHeight, true);
            if (overlayBitmap != sourceBitmap) {
                sourceBitmap.recycle();
            }
            sourceBitmap = null;

            float xRatio = (float) plan.getDouble("x_ratio");
            float bottomRatio = (float) plan.getDouble("bottom_ratio");
            float centerX = xRatio + targetWidth / (2f * dimensions[0]);
            float centerYFromBottom = bottomRatio + targetHeight / (2f * dimensions[1]);
            float anchorX = Math.max(-1f, Math.min(1f, centerX * 2f - 1f));
            float anchorY = Math.max(-1f, Math.min(1f, centerYFromBottom * 2f - 1f));
            File outputFile = new File(output);
            if (outputFile.exists() && !outputFile.delete()) {
                throw new IOException("无法覆盖旧的水印导出文件");
            }

            CountDownLatch completed = new CountDownLatch(1);
            AtomicReference<String> exportError = new AtomicReference<>();
            runOnUiThread(() -> startVideoWatermarkTransformer(
                input,
                outputFile,
                overlayBitmap,
                anchorX,
                anchorY,
                task.id,
                completed,
                exportError
            ));
            completed.await();
            if (exportError.get() != null) {
                throw new IOException(exportError.get());
            }
            updateTransferTask(task.id, "running", "视频水印已生成", 84, null, null, 0, null, null);

            JSONObject data = new JSONObject();
            data.put("message", "水印文件已导出");
            data.put("path", outputFile.getAbsolutePath());
            JSONObject response = new JSONObject();
            response.put("id", request.optLong("id"));
            response.put("command", "watermark");
            response.put("ok", true);
            response.put("data", data);
            response.put("error", JSONObject.NULL);
            return response.toString();
        } catch (Exception error) {
            if (sourceBitmap != null) {
                sourceBitmap.recycle();
            }
            throw error;
        }
    }

    private int[] videoDimensions(String input) throws IOException {
        MediaMetadataRetriever retriever = new MediaMetadataRetriever();
        try {
            retriever.setDataSource(input);
            int width = Integer.parseInt(
                retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_WIDTH)
            );
            int height = Integer.parseInt(
                retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_HEIGHT)
            );
            String rotationText = retriever.extractMetadata(MediaMetadataRetriever.METADATA_KEY_VIDEO_ROTATION);
            int rotation = rotationText == null ? 0 : Integer.parseInt(rotationText);
            if (rotation == 90 || rotation == 270) {
                int value = width;
                width = height;
                height = value;
            }
            if (width <= 0 || height <= 0) {
                throw new IOException("无法识别视频尺寸");
            }
            return new int[]{width, height};
        } catch (NumberFormatException error) {
            throw new IOException("无法识别视频尺寸", error);
        } finally {
            retriever.release();
        }
    }

    @androidx.media3.common.util.UnstableApi
    private void startVideoWatermarkTransformer(
        String input,
        File output,
        Bitmap overlayBitmap,
        float anchorX,
        float anchorY,
        long taskId,
        CountDownLatch completed,
        AtomicReference<String> exportError
    ) {
        try {
            if (activeVideoTransformer != null) {
                overlayBitmap.recycle();
                exportError.set("已有视频正在导出水印，请等待当前任务完成");
                completed.countDown();
                return;
            }
            StaticOverlaySettings settings = new StaticOverlaySettings.Builder()
                .setOverlayFrameAnchor(0f, 0f)
                .setBackgroundFrameAnchor(anchorX, anchorY)
                .build();
            BitmapOverlay bitmapOverlay = BitmapOverlay.createStaticBitmapOverlay(overlayBitmap, settings);
            OverlayEffect overlayEffect = new OverlayEffect(Collections.singletonList(bitmapOverlay));
            Effects effects = new Effects(
                Collections.emptyList(),
                Collections.singletonList(overlayEffect)
            );
            EditedMediaItem editedMediaItem = new EditedMediaItem.Builder(
                MediaItem.fromUri(Uri.fromFile(new File(input)))
            ).setEffects(effects).build();

            activeVideoTransformer = new Transformer.Builder(this)
                .setVideoMimeType(MimeTypes.VIDEO_H264)
                .addListener(new Transformer.Listener() {
                    @Override
                    public void onCompleted(Composition composition, ExportResult exportResult) {
                        activeVideoTransformer = null;
                        overlayBitmap.recycle();
                        completed.countDown();
                    }

                    @Override
                    public void onError(
                        Composition composition,
                        ExportResult exportResult,
                        ExportException exportException
                    ) {
                        activeVideoTransformer = null;
                        overlayBitmap.recycle();
                        output.delete();
                        exportError.set("视频水印导出失败：" + exportException.getMessage());
                        completed.countDown();
                    }
                })
                .build();
            activeVideoTransformer.start(editedMediaItem, output.getAbsolutePath());
            ProgressHolder progressHolder = new ProgressHolder();
            mainHandler.post(new Runnable() {
                @Override
                public void run() {
                    Transformer transformer = activeVideoTransformer;
                    if (completed.getCount() == 0 || transformer == null) {
                        return;
                    }
                    try {
                        int state = transformer.getProgress(progressHolder);
                        if (state == Transformer.PROGRESS_STATE_AVAILABLE) {
                            int progress = 8 + Math.round(progressHolder.progress * 0.74f);
                            updateTransferTask(
                                taskId,
                                "running",
                                "正在转码并写入视频水印",
                                Math.min(82, progress),
                                null,
                                null,
                                0,
                                null,
                                progressHolder.progress + "%"
                            );
                        }
                    } catch (Exception ignored) {
                    }
                    mainHandler.postDelayed(this, 350);
                }
            });
        } catch (Exception error) {
            activeVideoTransformer = null;
            overlayBitmap.recycle();
            output.delete();
            exportError.set("视频水印导出失败：" + error.getMessage());
            completed.countDown();
        }
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
        connection.setRequestProperty("User-Agent", "Insta360Linker Android/0.1");
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
            headers.put("User-Agent", "Insta360Linker Android/0.1");
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
            "window.Insta360LinkerBridge&&window.Insta360LinkerBridge.receive(" + response + ");",
            null
        ));
    }

    private String publishExportsToGallery(String responseText, TransferTask task) {
        try {
            JSONObject response = new JSONObject(responseText);
            if (!response.optBoolean("ok")) {
                return responseText;
            }
            String command = response.optString("command");
            JSONObject data = response.optJSONObject("data");
            if (data == null) {
                return responseText;
            }
            if ("watermark".equals(command)) {
                File source = new File(data.optString("path"));
                long phaseStartedAt = SystemClock.elapsedRealtime();
                Uri galleryUri = publishFileToGallery(source, (completed, total) -> {
                    int progress = total > 0
                        ? 84 + (int) Math.round((completed / (double) total) * 16.0)
                        : 84;
                    long elapsed = Math.max(1, SystemClock.elapsedRealtime() - phaseStartedAt);
                    long speed = completed * 1000L / elapsed;
                    Long eta = speed > 0 && total > completed ? (total - completed) / speed : 0L;
                    updateTransferTask(
                        task.id,
                        "running",
                        "正在写入系统相册",
                        progress,
                        completed,
                        total,
                        speed,
                        eta,
                        source.getName()
                    );
                });
                data.put("path", galleryUri.toString());
                data.put("message", "水印文件已保存到系统相册");
                deleteTemporaryExport(source);
            } else if ("download_batch".equals(command)) {
                JSONArray completed = data.optJSONArray("completed");
                JSONArray published = new JSONArray();
                JSONArray failed = data.optJSONArray("failed");
                if (failed == null) {
                    failed = new JSONArray();
                }
                long publishTotal = 0;
                if (completed != null) {
                    for (int index = 0; index < completed.length(); index++) {
                        JSONObject item = completed.optJSONObject(index);
                        if (item != null) {
                            publishTotal += new File(item.optString("output")).length();
                        }
                    }
                }
                final long totalBytes = publishTotal;
                long publishedBytes = 0;
                long phaseStartedAt = SystemClock.elapsedRealtime();
                if (completed != null) {
                    for (int index = 0; index < completed.length(); index++) {
                        JSONObject item = completed.optJSONObject(index);
                        if (item == null) {
                            continue;
                        }
                        File source = new File(item.optString("output"));
                        long sourceLength = source.length();
                        long completedBeforeFile = publishedBytes;
                        int itemNumber = index + 1;
                        int itemCount = completed.length();
                        try {
                            Uri galleryUri = publishFileToGallery(source, (copied, ignoredTotal) -> {
                                long aggregate = completedBeforeFile + copied;
                                int progress = totalBytes > 0
                                    ? 82 + (int) Math.round((aggregate / (double) totalBytes) * 18.0)
                                    : 82;
                                long elapsed = Math.max(1, SystemClock.elapsedRealtime() - phaseStartedAt);
                                long speed = aggregate * 1000L / elapsed;
                                Long eta = speed > 0 && totalBytes > aggregate
                                    ? (totalBytes - aggregate) / speed
                                    : 0L;
                                updateTransferTask(
                                    task.id,
                                    "running",
                                    "正在写入系统相册",
                                    progress,
                                    aggregate,
                                    totalBytes,
                                    speed,
                                    eta,
                                    itemNumber + "/" + itemCount + " · " + source.getName()
                                );
                            });
                            item.put("output", galleryUri.toString());
                            published.put(item);
                            deleteTemporaryExport(source);
                        } catch (Exception error) {
                            JSONObject failure = new JSONObject();
                            failure.put("name", item.optString("name", source.getName()));
                            failure.put("error", "保存到系统相册失败：" + error.getMessage());
                            failed.put(failure);
                        }
                        publishedBytes += sourceLength;
                    }
                }
                data.put("completed", published);
                data.put("failed", failed);
                data.put(
                    "message",
                    "已保存到系统相册：" + published.length() + " 个成功，" + failed.length() + " 个失败"
                );
            }
            return response.toString();
        } catch (Exception error) {
            try {
                JSONObject original = new JSONObject(responseText);
                return errorResponse(
                    original.optLong("id"),
                    original.optString("command"),
                    "保存到系统相册失败：" + error.getMessage()
                ).toString();
            } catch (Exception ignored) {
                return responseText;
            }
        }
    }

    private Uri publishFileToGallery(File source, FileProgress progress) throws IOException {
        if (!source.isFile()) {
            throw new IOException("导出文件不存在");
        }
        long totalBytes = source.length();
        progress.onProgress(0, totalBytes);
        boolean video = isVideoFile(source.getName());
        String mime = mediaMimeType(source.getName(), video);
        if (Build.VERSION.SDK_INT >= 29) {
            ContentResolver resolver = getContentResolver();
            Uri collection = video
                ? MediaStore.Video.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
                : MediaStore.Images.Media.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY);
            ContentValues values = new ContentValues();
            values.put(MediaStore.MediaColumns.DISPLAY_NAME, source.getName());
            values.put(MediaStore.MediaColumns.MIME_TYPE, mime);
            values.put(MediaStore.MediaColumns.RELATIVE_PATH, Environment.DIRECTORY_DCIM + "/Insta360Linker");
            values.put(MediaStore.MediaColumns.IS_PENDING, 1);
            Uri destination = resolver.insert(collection, values);
            if (destination == null) {
                throw new IOException("系统相册无法创建文件");
            }
            try {
                try (InputStream input = new FileInputStream(source);
                     OutputStream output = resolver.openOutputStream(destination)) {
                    if (output == null) {
                        throw new IOException("系统相册无法写入文件");
                    }
                    copyStream(input, output, totalBytes, progress);
                }
                ContentValues ready = new ContentValues();
                ready.put(MediaStore.MediaColumns.IS_PENDING, 0);
                resolver.update(destination, ready, null, null);
                return destination;
            } catch (Exception error) {
                resolver.delete(destination, null, null);
                if (error instanceof IOException) {
                    throw (IOException) error;
                }
                throw new IOException(error.getMessage(), error);
            }
        }

        if (checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) != PackageManager.PERMISSION_GRANTED) {
            throw new IOException("请允许存储权限后重试");
        }
        File album = new File(
            Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DCIM),
            "Insta360Linker"
        );
        if (!album.exists() && !album.mkdirs()) {
            throw new IOException("无法创建系统相册目录");
        }
        File destination = uniqueDestination(album, source.getName());
        try (InputStream input = new FileInputStream(source);
             OutputStream output = new FileOutputStream(destination)) {
            copyStream(input, output, totalBytes, progress);
        }
        MediaScannerConnection.scanFile(
            this,
            new String[]{destination.getAbsolutePath()},
            new String[]{mime},
            null
        );
        return Uri.fromFile(destination);
    }

    private void copyStream(
        InputStream input,
        OutputStream output,
        long totalBytes,
        FileProgress progress
    ) throws IOException {
        byte[] buffer = new byte[256 * 1024];
        int count;
        long completed = 0;
        long lastUpdate = 0;
        while ((count = input.read(buffer)) >= 0) {
            output.write(buffer, 0, count);
            completed += count;
            long now = SystemClock.elapsedRealtime();
            if (now - lastUpdate >= 120 || completed >= totalBytes) {
                progress.onProgress(completed, totalBytes);
                lastUpdate = now;
            }
        }
        output.flush();
        progress.onProgress(completed, totalBytes);
    }

    private interface FileProgress {
        void onProgress(long completedBytes, long totalBytes);
    }

    private File uniqueDestination(File directory, String name) {
        File candidate = new File(directory, safeName(name));
        if (!candidate.exists()) {
            return candidate;
        }
        String safe = safeName(name);
        int dot = safe.lastIndexOf('.');
        String stem = dot > 0 ? safe.substring(0, dot) : safe;
        String extension = dot > 0 ? safe.substring(dot) : "";
        int suffix = 2;
        do {
            candidate = new File(directory, stem + " (" + suffix++ + ")" + extension);
        } while (candidate.exists());
        return candidate;
    }

    private String watermarkOutputExtension(String input) {
        String lower = input == null ? "" : input.toLowerCase(Locale.ROOT);
        if (lower.endsWith(".mp4") || lower.endsWith(".mov") || lower.endsWith(".mkv")
            || lower.endsWith(".avi") || lower.endsWith(".m4v") || lower.endsWith(".insv")) {
            return ".mp4";
        }
        if (lower.endsWith(".png")) {
            return ".png";
        }
        if (lower.endsWith(".webp")) {
            return ".webp";
        }
        return ".jpg";
    }

    private boolean isVideoFile(String name) {
        String lower = name.toLowerCase(Locale.ROOT);
        return lower.endsWith(".mp4") || lower.endsWith(".mov") || lower.endsWith(".m4v")
            || lower.endsWith(".mkv") || lower.endsWith(".avi") || lower.endsWith(".insv");
    }

    private String mediaMimeType(String name, boolean video) {
        int dot = name.lastIndexOf('.');
        if (dot >= 0 && dot + 1 < name.length()) {
            String extension = name.substring(dot + 1).toLowerCase(Locale.ROOT);
            String detected = MimeTypeMap.getSingleton().getMimeTypeFromExtension(extension);
            if (detected != null) {
                return detected;
            }
            if ("insv".equals(extension)) {
                return "video/mp4";
            }
            if ("insp".equals(extension)) {
                return "image/jpeg";
            }
        }
        return video ? "video/mp4" : "image/jpeg";
    }

    private void deleteTemporaryExport(File source) {
        try {
            File exportRoot = new File(getCacheDir(), "exports").getCanonicalFile();
            File current = source.getCanonicalFile();
            if (!current.getPath().startsWith(exportRoot.getPath() + File.separator)) {
                return;
            }
            if (current.isFile()) {
                current.delete();
            }
            current = current.getParentFile();
            while (current != null && !current.equals(exportRoot)) {
                String[] children = current.list();
                if (children == null || children.length != 0 || !current.delete()) {
                    break;
                }
                current = current.getParentFile();
            }
        } catch (IOException ignored) {
        }
    }

    private void requestBluetoothPermissions() {
        List<String> missing = new ArrayList<>();
        if (Build.VERSION.SDK_INT <= 30) {
            if (checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
                missing.add(Manifest.permission.ACCESS_FINE_LOCATION);
            }
            if (Build.VERSION.SDK_INT <= 28
                && checkSelfPermission(Manifest.permission.WRITE_EXTERNAL_STORAGE) != PackageManager.PERMISSION_GRANTED) {
                missing.add(Manifest.permission.WRITE_EXTERNAL_STORAGE);
            }
        } else {
            if (checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) != PackageManager.PERMISSION_GRANTED) {
                missing.add(Manifest.permission.BLUETOOTH_SCAN);
            }
            if (checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) != PackageManager.PERMISSION_GRANTED) {
                missing.add(Manifest.permission.BLUETOOTH_CONNECT);
            }
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
