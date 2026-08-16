package studio.insta360.linker;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.io.ByteArrayOutputStream;
import org.junit.Test;

public final class HevcAccessUnitTest {
    @Test
    public void separatesCodecConfigFromIdrSample() {
        byte[] vps = nal(32, 0x11);
        byte[] sps = nal(33, 0x22);
        byte[] pps = nal(34, 0x33);
        byte[] idr = nal(19, 0x44);

        HevcAccessUnit.Parsed parsed = HevcAccessUnit.parse(join(vps, sps, pps, idr));

        assertTrue(parsed.hasCompleteCodecConfig);
        assertTrue(parsed.keyFrame);
        assertTrue(parsed.canStartDecoder());
        assertArrayEquals(join(vps, sps, pps), parsed.codecConfig);
        assertArrayEquals(idr, parsed.sample);
    }

    @Test
    public void preservesNonKeyFrameSample() {
        byte[] sample = nal(1, 0x55);

        HevcAccessUnit.Parsed parsed = HevcAccessUnit.parse(sample);

        assertFalse(parsed.hasCompleteCodecConfig);
        assertFalse(parsed.keyFrame);
        assertFalse(parsed.canStartDecoder());
        assertArrayEquals(sample, parsed.sample);
    }

    private static byte[] nal(int type, int marker) {
        return new byte[]{0, 0, 0, 1, (byte) (type << 1), 1, (byte) marker};
    }

    private static byte[] join(byte[]... values) {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        for (byte[] value : values) {
            output.write(value, 0, value.length);
        }
        return output.toByteArray();
    }
}
