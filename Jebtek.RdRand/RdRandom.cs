using System;
using System.IO;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Jebtek.RdRand
{
    public class RdRandom
    {
        private const int KEY_SIZE_API = 64;

        /// <summary>
        /// Checks for CPU compatibility with the RDRAND instruction. The RDRAND instruction 
        /// is only available on Intel IvyBridge and higher CPUs.
        /// </summary>
        /// <returns>true if CPU is compatible with RDRAND</returns>
        public static bool GeneratorAvailable()
        {
            return RdRandLib.GeneratorAvailable();
        }

        /// <summary>
        /// Returns a base64 encoded string of a series of random(perhaps ish) bytes. 
        /// 
        /// On Intel IvyBridge and higher CPUs, the byte array is truely random, 
        /// using the RDRAND assembly instruction to get the CPU to retrieve them.
        /// 
        /// On other processors, the byte array is pseudo random, using the current
        /// time as a seed value.
        /// </summary>
        /// <param name="length">Length (in bytes) / size of the byte array</param>
        /// <returns>Base64 encoded byte array</returns>
        public static string GenerateKey(int length)
        {
            string key = "";
            byte[] buffer = new byte[length];

            if (GeneratorAvailable())
            {
                buffer = GenerateBytes(length);
            }
            else
            {
                Random r = new Random();

                r.NextBytes(buffer);
            }

            key = Convert.ToBase64String(buffer);
            return key;
        }

        /// <summary>
        /// Uses the RDRAND instrution on Intel IvyBridge and higher CPUs
        /// to return truely random byte arrays. 
        /// 
        /// CAUTION: If this code is executed
        /// on non IvyBridge CPUs, it will return null!
        /// </summary>
        /// <param name="length">Size of the byte array</param>
        /// <returns>Random byte array, or null if no executing on compatible CPU</returns>
        public static byte[] GenerateBytes(int length)
        {
            if (!RdRandLib.GeneratorAvailable()) return null;

            int numInts = length / 4 + 1;
            int idx = 0;
            byte[] r = new byte[length];

            for (int i = 0; i < numInts; i++)
            {
                uint rnd = RdRandLib.Random32();
                byte[] tmp = IntToBytes(rnd);

                if (idx >= length) return r;
                r[idx++] = tmp[0];

                if (idx >= length) return r;
                r[idx++] = tmp[1];

                if (idx >= length) return r;
                r[idx++] = tmp[2];

                if (idx >= length) return r;
                r[idx++] = tmp[3];
            }

            return r;
        }

        /// <summary>
        /// Uses the RDRAND instrution on Intel IvyBridge and higher CPUs
        /// to return truely random integers. 
        /// 
        /// CAUTION: If this code is executed
        /// on non IvyBridge CPUs, it will return null!
        /// </summary>
        /// <returns>Random unsigned integer, or 0 if no executing on compatible CPU</returns>
        public static uint GenerateUnsignedInt()
        {
            if (!RdRandLib.GeneratorAvailable()) return 0;

            uint rnd = RdRandLib.Random32();

            return rnd;
        }

        /// <summary>
        /// Generates a 64 character long key that can be used as an API key. This can only be run on compatible Intel CPUs.
        /// </summary>
        /// <returns>64 random characters</returns>
        public static string GenerateAPIKey()
        {
            string key;

            //check if we are running on an Intel IvyBridge or higher CPU. Basically, it needs to have the rdrand instruction.
            if (!RdRandom.GeneratorAvailable())
                throw new Exception("This code can only be executed on Intel IvyBridge and higher CPUs");

            //should most likely be unique
            key = RdRandom.GenerateKey(KEY_SIZE_API);

            return key;
        }

        private static byte[] IntToBytes(uint input)
        {
            MemoryStream stream = new MemoryStream();
            using (BinaryWriter writer = new BinaryWriter(stream))
            {
                writer.Write(input);
            }

            byte[] bytes = stream.ToArray();
            return bytes;
        }
    }
}
