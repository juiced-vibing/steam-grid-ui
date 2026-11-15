using System.Windows;
using System.Windows.Controls;

namespace SteamGridUI;

public partial class MainWindow : Window
{
    private sealed record Target(User User, GameItem Game, ArtKind Kind);

    public MainWindow()
    {
        InitializeComponent();

        AssetsList.ItemsSource = _assets;

        // wired here, not in XAML: they would fire while the window is still being built
        UserBox.SelectionChanged += (_, _) => Run(LoadGames);
        GamesList.SelectionChanged += (_, _) => Run(() => LoadAssets(append: false));
        KindBox.SelectionChanged += (_, _) => Run(() => LoadAssets(append: false));
        AssetsList.SelectionChanged += (_, _) => UpdateButtons();
        SearchBox.TextChanged += (_, _) => RefreshFilter();
        SortBox.SelectionChanged += (_, _) => ApplySort();
        MatchBox.SelectionChanged += (_, _) => Run(PickMatch);
    }

    private ArtKind Kind => (ArtKind)((ComboBoxItem)KindBox.SelectedItem).Tag;

    private Target? Current =>
        UserBox.SelectedItem is User user && GamesList.SelectedItem is GameItem game ? new Target(user, game, Kind) : null;

    private void SetStatus(string text) => Status.Text = text;

    private void Report(Exception error)
    {
        switch (error)
        {
            case BackendException { Code: ErrorCode.NoApiKey } e:
                SetStatus(e.Message);
                AskApiKey();
                break;

            case BackendException e:
                SetStatus(e.Message);
                break;

            case DllNotFoundException:
                SetStatus("sgdb_core.dll is missing; it belongs next to SteamGridUI.exe");
                break;

            default:
                SetStatus($"Unexpected error: {error.Message}");
                break;
        }
    }

    private async void Run(Func<Task> work)
    {
        try
        {
            await work();
        }

        catch (Exception e)
        {
            Report(e);
        }
    }

    private void RunAction(Func<Task> work) => Run(async () =>
    {
        Actions.IsEnabled = false;

        try
        {
            await work();
        }

        finally
        {
            Actions.IsEnabled = true;
            UpdateButtons();
        }
    });

    private void OnLoaded(object sender, RoutedEventArgs e) => Run(async () =>
    {
        var init = await Native.Send(new Init());

        switch (init.Steam)
        {
            case SteamState.Missing missing:
                SetStatus(missing.Reason);
                break;

            case SteamState.Found { Users.Count: 0 }:
                SetStatus("No Steam accounts found. Sign in to Steam once, then restart this app.");
                break;

            case SteamState.Found found:
                UserBox.ItemsSource = found.Users;
                UserBox.SelectedIndex = 0; // most recently used account; loads its games
                break;
        }

        if (!init.ApiKeySet) AskApiKey();
    });

    private void OnApiKey(object sender, RoutedEventArgs e) => AskApiKey();

    private void AskApiKey()
    {
        if (new ApiKeyWindow { Owner = this }.ShowDialog() != true) return;

        SetStatus("API key saved.");
        Run(() => LoadAssets(append: false));
    }
}
