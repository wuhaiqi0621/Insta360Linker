package studio.insta360.linker;

import java.io.ByteArrayOutputStream;

final class HevcAccessUnit {
    private HevcAccessUnit() {}

    static Parsed parse(byte[] data) {
        ByteArrayOutputStream codecConfig = new ByteArrayOutputStream();
        ByteArrayOutputStream sample = new ByteArrayOutputStream(data.length);
        boolean hasVps = false;
        boolean hasSps = false;
        boolean hasPps = false;
        boolean keyFrame = false;

        int start = findStartCode(data, 0);
        if (start < 0) {
            return new Parsed(new byte[0], data.clone(), false, false);
        }

        while (start >= 0) {
            int startCodeLength = startCodeLength(data, start);
            int header = start + startCodeLength;
            int next = findStartCode(data, header + 2);
            int end = next >= 0 ? next : data.length;
            if (header < end) {
                int nalType = (data[header] & 0x7e) >> 1;
                if (nalType == 32 || nalType == 33 || nalType == 34) {
                    codecConfig.write(data, start, end - start);
                    hasVps |= nalType == 32;
                    hasSps |= nalType == 33;
                    hasPps |= nalType == 34;
                } else {
                    sample.write(data, start, end - start);
                    keyFrame |= nalType >= 16 && nalType <= 23;
                }
            }
            start = next;
        }

        return new Parsed(
            codecConfig.toByteArray(),
            sample.toByteArray(),
            hasVps && hasSps && hasPps,
            keyFrame
        );
    }

    private static int findStartCode(byte[] data, int from) {
        for (int index = Math.max(0, from); index + 2 < data.length; index++) {
            if (data[index] != 0 || data[index + 1] != 0) {
                continue;
            }
            if (data[index + 2] == 1) {
                return index;
            }
            if (index + 3 < data.length && data[index + 2] == 0 && data[index + 3] == 1) {
                return index;
            }
        }
        return -1;
    }

    private static int startCodeLength(byte[] data, int start) {
        return start + 3 < data.length && data[start + 2] == 0 && data[start + 3] == 1 ? 4 : 3;
    }

    static final class Parsed {
        final byte[] codecConfig;
        final byte[] sample;
        final boolean hasCompleteCodecConfig;
        final boolean keyFrame;

        Parsed(byte[] codecConfig, byte[] sample, boolean hasCompleteCodecConfig, boolean keyFrame) {
            this.codecConfig = codecConfig;
            this.sample = sample;
            this.hasCompleteCodecConfig = hasCompleteCodecConfig;
            this.keyFrame = keyFrame;
        }

        boolean canStartDecoder() {
            return hasCompleteCodecConfig && keyFrame && sample.length > 0;
        }
    }
}
