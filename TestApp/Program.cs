using System;
using System.IO;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Jebtek.RdRand;

namespace TestApp
{
    class Program
    {
        static void Main(string[] args)
        {
            Console.WriteLine("Generator Available: " + Jebtek.RdRandLib.GeneratorAvailable());
            uint i = Jebtek.RdRandLib.Random32();
            Console.WriteLine("Random Number: " + i);
            byte[] b = IntToBytes(i);
            Console.WriteLine("Base64: " + Convert.ToBase64String(b));

            //.NET Library helper demo
            Console.WriteLine("API Key: " + RdRandom.GenerateAPIKey());

            Console.ReadKey();
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
