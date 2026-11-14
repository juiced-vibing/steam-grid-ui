using System.Diagnostics;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Media;

namespace SteamGridUI;

/// <summary>Small dialog that collects and verifies the user's SteamGridDB API key.</summary>
public sealed class ApiKeyWindow : Window
{
    private const string KeyPage = "https://www.steamgriddb.com/profile/preferences/api";

    private readonly TextBox _key = new() { Margin = new Thickness(0, 8, 0, 0), Padding = new Thickness(4) };
    private readonly TextBlock _error = new() { Foreground = Brushes.Firebrick, TextWrapping = TextWrapping.Wrap, Margin = new Thickness(0, 6, 0, 0) };
    private readonly Button _ok = new() { Content = "Save", IsDefault = true, MinWidth = 80, Padding = new Thickness(8, 4, 8, 4) };

    public ApiKeyWindow()
    {
        Title = "SteamGridDB API key";
        Width = 460;
        SizeToContent = SizeToContent.Height;
        ResizeMode = ResizeMode.NoResize;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        ShowInTaskbar = false;

        var link = new Hyperlink(new Run("your SteamGridDB preferences page"));
        link.Click += (_, _) => Process.Start(new ProcessStartInfo(KeyPage) { UseShellExecute = true });
        var intro = new TextBlock { TextWrapping = TextWrapping.Wrap };
        intro.Inlines.Add("Artwork is fetched through SteamGridDB's API with your own free key. Create one on ");
        intro.Inlines.Add(link);
        intro.Inlines.Add(" and paste it here.");

        var cancel = new Button { Content = "Cancel", IsCancel = true, MinWidth = 80, Margin = new Thickness(8, 0, 0, 0), Padding = new Thickness(8, 4, 8, 4) };
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 12, 0, 0) };
        buttons.Children.Add(_ok);
        buttons.Children.Add(cancel);
        _ok.Click += OnSave;

        var panel = new StackPanel { Margin = new Thickness(16) };
        panel.Children.Add(intro);
        panel.Children.Add(_key);
        panel.Children.Add(_error);
        panel.Children.Add(buttons);
        Content = panel;
        Loaded += (_, _) => _key.Focus();
    }

    private async void OnSave(object sender, RoutedEventArgs e)
    {
        var key = _key.Text.Trim();
        if (key.Length == 0)
        {
            _error.Text = "Paste your API key first.";
            return;
        }

        _ok.IsEnabled = false;
        _error.Text = "";
        try
        {
            await Native.Call("set_api_key", new { key });
            DialogResult = true;
        }
        catch (Exception ex)
        {
            _error.Text = ex.Message;
            _ok.IsEnabled = true;
        }
    }
}
