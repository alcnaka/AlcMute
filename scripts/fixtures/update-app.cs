using System;
using System.IO;
using System.Reflection;
using System.Threading;
[assembly: AssemblyCompany("Alcnaka")]
[assembly: AssemblyProduct("AlcMute Local Update Fixture")]
[assembly: AssemblyFileVersion("FIXTURE_VERSION")]
[assembly: AssemblyInformationalVersion("FIXTURE_VERSION")]
class Fixture {
    static string NormalPath(string path) {
        return path.StartsWith(@"\\?\") ? path.Substring(4) : path;
    }
    static void Main(string[] args) {
        if (args.Length == 2 && args[0] == "--wait") {
            while (!File.Exists(NormalPath(args[1]))) Thread.Sleep(10);
        } else if (args.Length == 3 && args[0] == "--background" && args[1] == "--data-dir") {
            File.WriteAllText(Path.Combine(NormalPath(args[2]), "restarted-version.txt"), "FIXTURE_VERSION");
        }
    }
}

