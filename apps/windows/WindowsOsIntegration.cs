using System.Diagnostics;
using Windows.ApplicationModel.DataTransfer;

namespace ReviewRadar.Windows;

public interface IOsIntegration
{
    string ApplicationDataDirectory { get; }

    void OpenUrl(Uri url);

    void CopyText(string text);
}

public sealed class WindowsOsIntegration : IOsIntegration
{
    public WindowsOsIntegration()
    {
        ApplicationDataDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            "review-radar");
        Directory.CreateDirectory(ApplicationDataDirectory);
    }

    public string ApplicationDataDirectory { get; }

    public void OpenUrl(Uri url)
    {
        Process.Start(new ProcessStartInfo(url.AbsoluteUri) { UseShellExecute = true });
    }

    public void CopyText(string text)
    {
        var package = new DataPackage();
        package.SetText(text);
        Clipboard.SetContent(package);
    }
}
