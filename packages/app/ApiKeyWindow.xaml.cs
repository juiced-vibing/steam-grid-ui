using System.Diagnostics;
using System.Windows;
using System.Windows.Navigation;

namespace SteamGridUI;

public partial class ApiKeyWindow : Window
{
    public ApiKeyWindow() => InitializeComponent();

    private void OnOpenLink(object sender, RequestNavigateEventArgs e) =>
        Process.Start(new ProcessStartInfo(e.Uri.AbsoluteUri) { UseShellExecute = true });

    private async void OnSave(object sender, RoutedEventArgs e)
    {
        var key = Key.Text.Trim();
        if (key.Length == 0)
        {
            Error.Text = "Paste your API key first.";
            return;
        }

        Ok.IsEnabled = false;
        Error.Text = "";

        try
        {
            await Native.Send(new SetApiKey(key));
            DialogResult = true;
        }

        // rejected key, no network, rate limit: the message says which
        catch (BackendException ex)
        {
            Error.Text = ex.Message;
            Ok.IsEnabled = true;
        }
    }
}
