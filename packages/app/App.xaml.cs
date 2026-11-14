using System.Windows;
using System.Windows.Threading;

namespace SteamGridUI;

public partial class App : Application
{
    private void OnUnhandled(object sender, DispatcherUnhandledExceptionEventArgs e)
    {
        MessageBox.Show(e.Exception.Message, "Steam Grid UI", MessageBoxButton.OK, MessageBoxImage.Error);
        e.Handled = true;
    }
}
