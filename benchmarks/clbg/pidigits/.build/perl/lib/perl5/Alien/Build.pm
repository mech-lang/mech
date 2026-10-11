package Alien::Build;

use strict;
use warnings;
use 5.008004;
use Path::Tiny ();
use Carp ();
use File::chdir;
use JSON::PP ();
use Env qw( @PATH @PKG_CONFIG_PATH );
use Config ();
use Alien::Build::Log;

# ABSTRACT: Build external dependencies for use in CPAN
our $VERSION = '2.88'; # VERSION


sub _path { goto \&Path::Tiny::path }


sub new
{
  my($class, %args) = @_;
  my $self = bless {
    install_prop => {
      root  => _path($args{root} || "_alien")->absolute->stringify,
      patch => (defined $args{patch}) ? _path($args{patch})->absolute->stringify : undef,
    },
    runtime_prop => {
      alien_build_version => $Alien::Build::VERSION || 'dev',
    },
    plugin_instance_prop => {},
    bin_dir => [],
    pkg_config_path => [],
    aclocal_path => [],
  }, $class;

  # force computing this as soon as possible
  $self->download_rule;

  $self->meta->filename(
    $args{filename} || do {
      my(undef, $filename) = caller;
      _path($filename)->absolute->stringify;
    }
  );

  if($args{meta_prop})
  {
    $self->meta->prop->{$_} = $args{meta_prop}->{$_} for keys %{ $args{meta_prop} };
  }

  $self;
}


my $count = 0;

sub load
{
  my(undef, $alienfile, @args) = @_;

  my $rcfile = Path::Tiny->new($ENV{ALIEN_BUILD_RC} || '~/.alienbuild/rc.pl')->absolute;
  if(-r $rcfile)
  {
    require Alien::Build::rc;
    package Alien::Build::rc;
    require $rcfile;
  }

  unless(-r $alienfile)
  {
    Carp::croak "Unable to read alienfile: $alienfile";
  }

  my $file = _path $alienfile;
  my $name = $file->parent->basename;
  $name =~ s/^alien-//i;
  $name =~ s/[^a-z]//g;
  $name = 'x' if $name eq '';
  $name = ucfirst $name;

  my $class = "Alien::Build::Auto::$name@{[ $count++ ]}";

  { no strict 'refs';
  @{ "${class}::ISA" } = ('Alien::Build');
  *{ "${class}::Alienfile::meta" } = sub {
    $class =~ s{::Alienfile$}{};
    $class->meta;
  }};

  my @preload = qw( Core::Setup Core::Download Core::FFI Core::Override Core::CleanInstall );
  push @preload, @Alien::Build::rc::PRELOAD;
  push @preload, split /;/, $ENV{ALIEN_BUILD_PRELOAD}
    if defined $ENV{ALIEN_BUILD_PRELOAD};

  my @postload = qw( Core::Legacy Core::Gather Core::Tail );
  push @postload, @Alien::Build::rc::POSTLOAD;
  push @postload, split /;/, $ENV{ALIEN_BUILD_POSTLOAD}
    if defined $ENV{ALIEN_BUILD_POSTLOAD};

  my $self = $class->new(
    filename => $file->absolute->stringify,
    @args,
  );

  require alienfile;

  foreach my $preload (@preload)
  {
    ref $preload eq 'CODE' ? $preload->($self->meta) : $self->meta->apply_plugin($preload);
  }

  # TODO: do this without a string eval ?
  ## no critic
  eval '# line '. __LINE__ . ' "' . __FILE__ . qq("\n) . qq{
    package ${class}::Alienfile;
    do '@{[ $file->absolute->stringify ]}';
    die \$\@ if \$\@;
  };
  die $@ if $@;
  ## use critic

  foreach my $postload (@postload)
  {
    ref $postload eq 'CODE' ? $postload->($self->meta) : $self->meta->apply_plugin($postload);
  }

  $self->{args} = \@args;
  unless(defined $self->meta->prop->{arch})
  {
    $self->meta->prop->{arch} = 1;
  }

  unless(defined $self->meta->prop->{network})
  {
    $self->meta->prop->{network} = 1;
    ## https://github.com/PerlAlien/Alien-Build/issues/23#issuecomment-341114414
    #$self->meta->prop->{network} = 0 if $ENV{NO_NETWORK_TESTING};
    $self->meta->prop->{network} = 0 if (defined $ENV{ALIEN_INSTALL_NETWORK}) && ! $ENV{ALIEN_INSTALL_NETWORK};
  }

  unless(defined $self->meta->prop->{local_source})
  {
    if(! defined $self->meta->prop->{start_url})
    {
      $self->meta->prop->{local_source} = 0;
    }
    # we assume URL schemes are at least two characters, that
    # way Windows absolute paths can be used as local start_url
    elsif($self->meta->prop->{start_url} =~ /^([a-z]{2,}):/i)
    {
      my $scheme = $1;
      $self->meta->prop->{local_source} = $scheme eq 'file';
    }
    else
    {
      $self->meta->prop->{local_source} = 1;
    }
  }

  return $self;
}


sub resume
{
  my(undef, $alienfile, $root) = @_;
  my $h = JSON::PP::decode_json(_path("$root/state.json")->slurp);
  my $self = Alien::Build->load("$alienfile", @{ $h->{args} });
  $self->{install_prop}         = $h->{install};
  $self->{plugin_instance_prop} = $h->{plugin_instance};
  $self->{runtime_prop}         = $h->{runtime};
  $self;
}


sub meta_prop
{
  my($class) = @_;
  $class->meta->prop;
}


sub install_prop
{
  shift->{install_prop};
}


sub plugin_instance_prop
{
  my($self, $plugin) = @_;
  my $instance_id = $plugin->instance_id;
  $self->{plugin_instance_prop}->{$instance_id} ||= {};
}


sub runtime_prop
{
  shift->{runtime_prop};
}


sub hook_prop
{
  shift->{hook_prop};
}

sub _command_prop
{
  my($self) = @_;

  return {
    alien => {
      install => $self->install_prop,
      runtime => $self->runtime_prop,
      hook    => $self->hook_prop,
      meta    => $self->meta_prop,
    },
    perl => {
      config => \%Config::Config,
    },
    env => \%ENV,
  };
}


sub checkpoint
{
  my($self) = @_;
  my $root = $self->root;
  _path("$root/state.json")->spew(
    JSON::PP->new->pretty->canonical(1)->ascii->encode({
      install         => $self->install_prop,
      runtime         => $self->runtime_prop,
      plugin_instance => $self->{plugin_instance_prop},
      args            => $self->{args},
    })
  );
  $self;
}


sub root
{
  my($self) = @_;
  my $root = $self->install_prop->{root};
  _path($root)->mkpath unless -d $root;
  $root;
}


sub install_type
{
  my($self) = @_;
  $self->{runtime_prop}->{install_type} ||= $self->probe;
}


sub is_system_install
{
  my($self) = @_;
  $self->install_type eq 'system';
}


sub is_share_install
{
  my($self) = @_;
  $self->install_type eq 'share';
}



sub download_rule
{
  my($self) = @_;

  $self->install_prop->{download_rule} ||= do {
    my $dr = $ENV{ALIEN_DOWNLOAD_RULE};
    $dr = 'warn' unless defined $dr;
    $dr = 'warn' if $dr eq 'default';
    unless($dr =~ /^(warn|digest|encrypt|digest_or_encrypt|digest_and_encrypt)$/)
    {
      $self->log("unknown ALIEN_DOWNLOAD_RULE \"$dr\", using \"warn\" instead");
      $dr = 'warn';
    }
    $dr;
  };
}


sub set_prefix
{
  my($self, $prefix) = @_;

  if($self->meta_prop->{destdir})
  {
    $self->runtime_prop->{prefix} =
    $self->install_prop->{prefix} = $prefix;
  }
  else
  {
    $self->runtime_prop->{prefix} = $prefix;
    $self->install_prop->{prefix} = $self->install_prop->{stage};
  }
}


sub set_stage
{
  my($self, $dir) = @_;
  $self->install_prop->{stage} = $dir;
}

sub _merge
{
  my %h;
  while(@_)
  {
    my $mod = shift;
    my $ver = shift;
    if((!defined $h{$mod}) || $ver > $h{$mod})
    { $h{$mod} = $ver }
  }
  \%h;
}


sub requires
{
  my($self, $phase) = @_;
  $phase ||= 'any';
  my $meta = $self->meta;
  $phase =~ /^(?:any|configure)$/
  ? $meta->{require}->{$phase} || {}
  : _merge %{ $meta->{require}->{any} }, %{ $meta->{require}->{$phase} };
}


sub load_requires
{
  my($self, $phase, $eval) = @_;
  my $reqs = $self->requires($phase);
  foreach my $mod (keys %$reqs)
  {
    my $ver = $reqs->{$mod};
    my $check = sub {
      my $pm = "$mod.pm";
      $pm =~ s{::}{/}g;
      require $pm;
    };
    if($eval)
    {
      eval { $check->() };
      die "Required $mod @{[ $ver || 'undef' ]}, missing" if $@;
    }
    else
    {
      $check->();
    }
    # note Test::Alien::Build#alienfile_skip_if_missing_prereqs does a regex
    # on this diagnostic, so if you change it here, change it there too.
    die "Required $mod $ver, have @{[ $mod->VERSION || 0 ]}" if $ver && ! $mod->VERSION($ver);

    # allow for requires on Alien::Build or Alien::Base
    next if $mod eq 'Alien::Build';
    next if $mod eq 'Alien::Base';

    if($mod->can('bin_dir'))
    {
      push @{ $self->{bin_dir} }, $mod->bin_dir;
    }

    if(($mod->can('runtime_prop') && $mod->runtime_prop)
    || ($mod->isa('Alien::Base')  && $mod->install_type('share')))
    {
      for my $dir (qw(lib share)) {
          my $path = _path($mod->dist_dir)->child("$dir/pkgconfig");
          if(-d $path)
          {
            push @{ $self->{pkg_config_path} }, $path->stringify;
          }
      }
      my $path = _path($mod->dist_dir)->child('share/aclocal');
      if(-d $path)
      {
        $path = "$path";
        if($^O eq 'MSWin32')
        {
          # convert to MSYS path
          $path =~ s{^([a-z]):}{/$1/}i;
        }
        push @{ $self->{aclocal_path} }, $path;
      }
    }

    # sufficiently new Autotools have a aclocal_dir which will
    # give us the directories we need.
    if($mod eq 'Alien::Autotools' && $mod->can('aclocal_dir'))
    {
      push @{ $self->{aclocal_path} }, $mod->aclocal_dir;
    }

    if($mod->can('alien_helper'))
    {
      my $helpers = $mod->alien_helper;
      foreach my $name (sort keys %$helpers)
      {
        my $code = $helpers->{$name};
        $self->meta->interpolator->replace_helper($name => $code);
      }
    }

  }
  1;
}

sub _call_hook
{
  my $self = shift;

  local $ENV{PATH} = $ENV{PATH};
  unshift @PATH, @{ $self->{bin_dir} };

  local $ENV{PKG_CONFIG_PATH} = $ENV{PKG_CONFIG_PATH};
  unshift @PKG_CONFIG_PATH, @{ $self->{pkg_config_path} };

  local $ENV{ACLOCAL_PATH} = $ENV{ACLOCAL_PATH};
  # autoconf uses MSYS paths, even for the ACLOCAL_PATH environment variable, so we can't use Env for this.
  {
    my @path;
    @path = split /:/, $ENV{ACLOCAL_PATH} if defined $ENV{ACLOCAL_PATH};
    unshift @path, @{ $self->{aclocal_path} };
    $ENV{ACLOCAL_PATH} = join ':', @path;
  }

  my $config = ref($_[0]) eq 'HASH' ? shift : {};
  my($name, @args) = @_;

  local $self->{hook_prop} = {};

  $self->meta->call_hook( $config, $name => $self, @args );
}


sub probe
{
  my($self) = @_;
  local $CWD = $self->root;
  my $dir;

  my $env = $self->_call_hook('override');
  my $type;
  my $error;

  $env = '' if $env eq 'default';

  if($env eq 'share')
  {
    $type = 'share';
  }
  else
  {
    $type = eval {
      $self->_call_hook(
        {
          before   => sub {
            $dir = Alien::Build::TempDir->new($self, "probe");
            $CWD = "$dir";
          },
          after    => sub {
            $CWD = $self->root;
          },
          ok       => 'system',
          continue => sub {
            if($_[0] eq 'system')
            {
              foreach my $name (qw( probe_class probe_instance_id ))
              {
                if(exists $self->hook_prop->{$name} && defined $self->hook_prop->{$name})
                {
                  $self->install_prop->{"system_$name"} = $self->hook_prop->{$name};
                }
              }
              return undef;
            }
            else
            {
              return 1;
            }
          },
        },
        'probe',
      );
    };
    $error = $@;
    $type = 'share' unless defined $type;
  }

  if($error)
  {
    if($env eq 'system')
    {
      die $error;
    }
    $self->log("error in probe (will do a share install): $@");
    $self->log("Don't panic, we will attempt a share build from source if possible.");
    $self->log("Do not file a bug unless you expected a system install to succeed.");
    $type = 'share';
  }

  if($env && $env ne $type)
  {
    die "requested $env install not available";
  }

  if($type !~ /^(system|share)$/)
  {
    Carp::croak "probe hook returned something other than system or share: $type";
  }

  if($type eq 'share' && (!$self->meta_prop->{network}) && (!$self->meta_prop->{local_source}))
  {
    $self->log("install type share requested or detected, but network fetch is turned off");
    $self->log("see https://metacpan.org/pod/Alien::Build::Manual::FAQ#Network-fetch-is-turned-off");
    Carp::croak "network fetch is turned off";
  }

  $self->runtime_prop->{install_type} = $type;

  $type;
}


sub download
{
  my($self) = @_;

  return $self unless $self->install_type eq 'share';
  return $self if $self->install_prop->{complete}->{download};

  if($self->meta->has_hook('download'))
  {
    my $tmp;
    local $CWD;
    my $valid = 0;

    $self->_call_hook(
      {
        before => sub {
          $tmp = Alien::Build::TempDir->new($self, "download");
          $CWD = "$tmp";
        },
        verify => sub {
          my @list = grep { $_->basename !~ /^\./, } _path('.')->children;

          my $count = scalar @list;

          if($count == 0)
          {
            die "no files downloaded";
          }
          elsif($count == 1)
          {
            my($archive) = $list[0];
            if(-d $archive)
            {
              # TODO: this is probably a bug that we don't set
              # download or compelte properties?
              $self->log("single dir, assuming directory");
            }
            else
            {
              $self->log("single file, assuming archive");
            }
            $self->install_prop->{download} = $archive->absolute->stringify;
            $self->install_prop->{complete}->{download} = 1;
            $valid = 1;
          }
          else
          {
            $self->log("multiple files, assuming directory");
            $self->install_prop->{complete}->{download} = 1;
            $self->install_prop->{download} = _path('.')->absolute->stringify;
            $valid = 1;
          }
        },
        after  => sub {
          $CWD = $self->root;
        },
      },
      'download',
    );

    # experimental and undocumented for now
    if($self->meta->has_hook('check_download'))
    {
      $self->meta->call_hook(check_download => $self);
    }

    return $self if $valid;
  }
  else
  {
    # This will call the default download hook
    # defined in Core::Download since the recipe
    # does not provide a download hook
    my $ret = $self->_call_hook('download');

    # experimental and undocumented for now
    if($self->meta->has_hook('check_download'))
    {
      $self->meta->call_hook(check_download => $self);
    }

    return $self;
  }

  die "download failed";
}


sub fetch
{
  my $self = shift;
  my $url = $_[0] || $self->meta_prop->{start_url};

  my $secure = 0;

  if(defined $url && ($url =~ /^(https|file):/ || $url !~ /:/))
  {
    # considered secure when either https or a local file
    $secure = 1;
  }
  elsif(!defined $url)
  {
    $self->log("warning: undefined url in fetch");
  }
  else
  {
    $self->log("warning: attempting to fetch a non-TLS or bundled URL: @{[ $url ]}");
  }

  die "insecure fetch is not allowed" if $self->download_rule =~ /^(encrypt|digest_and_encrypt)$/ && !$secure;

  my $file = $self->_call_hook( 'fetch' => @_ );

  $secure = 0;

  if(ref($file) ne 'HASH')
  {
    $self->log("warning: fetch returned non-hash reference");
  }
  elsif(!defined $file->{protocol})
  {
    $self->log("warning: fetch did not return a protocol");
  }
  elsif($file->{protocol} !~ /^(https|file)$/)
  {
    $self->log("warning: fetch did not use a secure protocol: @{[ $file->{protocol} ]}");
  }
  else
  {
    $secure = 1;
  }

  die "insecure fetch is not allowed" if $self->download_rule =~ /^(encrypt|digest_and_encrypt)$/ && !$secure;

  $file;
}


sub check_digest
{
  my($self, $file) = @_;

  return '' unless $self->meta_prop->{check_digest};

  unless(ref($file) eq 'HASH')
  {
    my $path = Path::Tiny->new($file);
    $file = {
      type     => 'file',
      filename => $path->basename,
      path     => "$path",
      tmp      => 0,
    };
  }

  my $path = $file->{path};
  if(defined $path)
  {
    # there is technically a race condition here
    die "Missing file in digest check: @{[ $file->{filename} ]}" unless -f $path;
    die "Unreadable file in digest check: @{[ $file->{filename} ]}" unless -r $path;
  }
  else
  {
    die "File is wrong type" unless defined $file->{type} && $file->{type} eq 'file';
    die "File has no filename" unless defined $file->{filename};
    die "@{[ $file->{filename} ]} has no content" unless defined $file->{content};
  }

  my $filename = $file->{filename};
  my $signature = $self->meta_prop->{digest}->{$filename} || $self->meta_prop->{digest}->{'*'};

  die "No digest for $filename" unless defined $signature && ref $signature eq 'ARRAY';

  my($algo, $expected) = @$signature;

  if($self->meta->call_hook( check_digest => $self, $file, $algo, $expected ))
  {
    # record the verification here so that we can check in the extract step that the signature
    # was checked.
    $self->install_prop->{download_detail}->{$path}->{digest} = [$algo, $expected] if defined $path; return 1;
  }
  else
  {
    die "No plugin provides digest algorithm for $algo";
  }
}


sub decode
{
  my($self, $res) = @_;
  my $res2 = $self->_call_hook( decode => $res );
  $res2->{protocol} = $res->{protocol}
    if !defined $res2->{protocol}
    &&  defined $res->{protocol};
  return $res2;
}


sub prefer
{
  my($self, $res) = @_;
  my $res2 = $self->_call_hook( prefer => $res );
  $res2->{protocol} = $res->{protocol}
    if !defined $res2->{protocol}
    &&  defined $res->{protocol};
  return $res2;
}


sub extract
{
  my($self, $archive) = @_;

  $archive ||= $self->install_prop->{download};

  unless(defined $archive)
  {
    die "tried to call extract before download";
  }

  {
    my $checked_digest  = 0;
    my $encrypted_fetch = 0;
    my $detail = $self->install_prop->{download_detail}->{$archive};
    if(defined $detail)
    {
      if(defined $detail->{digest})
      {
        my($algo, $expected) = @{ $detail->{digest} };
        my $file = {
          type     => 'file',
          filename => Path::Tiny->new($archive)->basename,
          path     => $archive,
          tmp      => 0,
        };
        $checked_digest = $self->meta->call_hook( check_digest => $self, $file, $algo, $expected )
      }
      if(!defined $detail->{protocol})
      {
        $self->log("warning: extract did not receive protocol details for $archive") unless $checked_digest;
      }
      elsif($detail->{protocol} !~ /^(https|file)$/)
      {
        $self->log("warning: extracting from a file that was fetched via insecure protocol @{[ $detail->{protocol} ]}") unless $checked_digest ;
      }
      else
      {
        $encrypted_fetch = 1;
      }
    }
    else
    {
      $self->log("warning: extract received no download details for $archive");
    }

    if($self->download_rule eq 'digest')
    {
      die "required digest missing for $archive" unless $checked_digest;
    }
    elsif($self->download_rule eq 'encrypt')
    {
      die "file was fetched insecurely for $archive" unless $encrypted_fetch;
    }
    elsif($self->download_rule eq 'digest_or_encrypt')
    {
      die "file was fetched insecurely and required digest missing for $archive" unless $checked_digest || $encrypted_fetch;
    }
    elsif($self->download_rule eq 'digest_and_encrypt')
    {
      die "file was fetched insecurely and required digest missing for $archive" unless $checked_digest || $encrypted_fetch;
      die "required digest missing for $archive" unless $checked_digest;
      die "file was fetched insecurely for $archive" unless $encrypted_fetch;
    }
    elsif($self->download_rule eq 'warn')
    {
      unless($checked_digest || $encrypted_fetch)
      {
        $self->log("!!! NOTICE OF FUTURE CHANGE IN BEHAVIOR !!!");
        $self->log("a future version of Alien::Build will die here by default with this exception: file was fetched insecurely and required digest missing for $archive");
        $self->log("!!! NOTICE OF FUTURE CHANGE IN BEHAVIOR !!!");
      }
    }
    else
    {
      die "internal error, unknown download rule: @{[ $self->download_rule ]}";
    }

  }

  my $nick_name = 'build';

  if($self->meta_prop->{out_of_source})
  {
    $nick_name = 'extract';
    my $extract = $self->install_prop->{extract};
    return $extract if defined $extract && -d $extract;
  }

  my $tmp;
  local $CWD;
  my $ret;

  $self->_call_hook({

    before => sub {
      # called build instead of extract, because this
      # will be used for the build step, and technically
      # extract is a substage of build anyway.
      $tmp = Alien::Build::TempDir->new($self, $nick_name);
      $CWD = "$tmp";
    },
    verify => sub {

      my $path = '.';
      if($self->meta_prop->{out_of_source} && $self->install_prop->{extract})
      {
        $path = $self->install_prop->{extract};
      }

      my @list = grep { $_->basename !~ /^\./ && $_->basename ne 'pax_global_header' } _path($path)->children;

      my $count = scalar @list;

      if($count == 0)
      {
        die "no files extracted";
      }
      elsif($count == 1 && -d $list[0])
      {
        $ret = $list[0]->absolute->stringify;
      }
      else
      {
        $ret = "$tmp";
      }

    },
    after => sub {
      $CWD = $self->root;
    },

  }, 'extract', $archive);

  $self->install_prop->{extract} ||= $ret;
  $ret ? $ret : ();
}


sub build
{
  my($self) = @_;

  # save the evironment, in case some plugins decide
  # to alter it.  Or us!  See just a few lines below.
  local %ENV = %ENV;

  my $stage = _path($self->install_prop->{stage});
  $stage->mkpath;

  my $tmp;

  if($self->install_type eq 'share')
  {
    foreach my $suffix ('', '_ffi')
    {
      local $CWD;
      delete $ENV{DESTDIR} unless $self->meta_prop->{destdir};

      my %env_meta = %{ $self->meta_prop   ->{env} || {} };
      my %env_inst = %{ $self->install_prop->{env} || {} };

      if($self->meta_prop->{env_interpolate})
      {
        foreach my $key (keys %env_meta)
        {
          $env_meta{$key} = $self->meta->interpolator->interpolate($env_meta{$key}, $self);
        }
      }

      %ENV = (%ENV, %env_meta);
      %ENV = (%ENV, %env_inst);

      my $destdir;

      $self->_call_hook(
      {
        before => sub {
          if($self->meta_prop->{out_of_source})
          {
            $self->extract;
            $CWD = $tmp = Alien::Build::TempDir->new($self, 'build');
          }
          else
          {
            $CWD = $tmp = $self->extract;
          }
          if($self->meta_prop->{destdir})
          {
            $destdir = Alien::Build::TempDir->new($self, 'destdir');
            $ENV{DESTDIR} = "$destdir";
          }
          $self->_call_hook({ all => 1 }, "patch${suffix}");
        },
        after => sub {
          $destdir = "$destdir" if $destdir;
        },
      }, "build${suffix}");

      $self->install_prop->{"_ab_build@{[ $suffix || '_share' ]}"} = "$CWD";

      $self->_call_hook("gather@{[ $suffix || '_share' ]}");
    }
  }

  elsif($self->install_type eq 'system')
  {
    local $CWD = $self->root;
    my $dir;

    $self->_call_hook(
      {
        before => sub {
          $dir = Alien::Build::TempDir->new($self, "gather");
          $CWD = "$dir";
        },
        after  => sub {
          $CWD = $self->root;
        },
      },
      'gather_system',
    );

    $self->install_prop->{finished} = 1;
    $self->install_prop->{complete}->{gather_system} = 1;
  }

  $self;
}


sub test
{
  my($self) = @_;

  if($self->install_type eq 'share')
  {
    foreach my $suffix ('_share', '_ffi')
    {
      if($self->meta->has_hook("test$suffix"))
      {
        my $dir = $self->install_prop->{"_ab_build$suffix"};
        Carp::croak("no build directory to run tests") unless $dir && -d $dir;
        local $CWD = $dir;
        $self->_call_hook("test$suffix");
      }
    }
  }
  else
  {
    if($self->meta->has_hook("test_system"))
    {
      my $dir = Alien::Build::TempDir->new($self, "test");
      local $CWD = "$dir";
      $self->_call_hook("test_system");
    }
  }

}


sub clean_install
{
  my($self) = @_;
  $self->_call_hook("clean_install");
}


sub system
{
  my($self, $command, @args) = @_;

  my $prop = $self->_command_prop;

  ($command, @args) = map {
    $self->meta->interpolator->interpolate($_, $prop)
  } ($command, @args);

  $self->log("+ $command @args");

  scalar @args
    ? system $command, @args
    : system $command;
}


sub log
{
  my(undef, $message) = @_;
  my $caller = [caller];
  chomp $message;
  foreach my $line (split /\n/, $message)
  {
    Alien::Build::Log->default->log(
      caller  => $caller,
      message => $line,
    );
  }
}


{
  my %meta;

  sub meta
  {
    my($class) = @_;
    $class = ref $class if ref $class;
    $meta{$class} ||= Alien::Build::Meta->new( class => $class );
  }
}

package Alien::Build::Meta;

our @CARP_NOT = qw( alienfile );

sub new
{
  my($class, %args) = @_;
  my $self = bless {
    phase => 'any',
    build_suffix => '',
    require => {
      any    => {},
      share  => {},
      system => {},
    },
    around => {},
    prop => {},
    %args,
  }, $class;
  $self;
}


sub prop
{
  shift->{prop};
}

sub filename
{
  my($self, $new) = @_;
  $self->{filename} = $new if defined $new;
  $self->{filename};
}


sub add_requires
{
  my $self = shift;
  my $phase = shift;
  while(@_)
  {
    my $module = shift;
    my $version = shift;
    my $old = $self->{require}->{$phase}->{$module};
    if((!defined $old) || $version > $old)
    { $self->{require}->{$phase}->{$module} = $version }
  }
  $self;
}


sub has_requires
{
  my $self = shift;
  my $phase = shift;
  my @has = keys %{ $self->{require}->{$phase} };
  while(@_)
  {
    my $pattern = shift;
    for (@has) { return 1 if index($_, $pattern) != -1; }
  }
  return ();
}


sub interpolator
{
  my($self, $new) = @_;
  if(defined $new)
  {
    if(defined $self->{intr})
    {
      Carp::croak "tried to set interpolator twice";
    }
    if(ref $new)
    {
      $self->{intr} = $new;
    }
    else
    {
      $self->{intr} = $new->new;
    }
  }
  elsif(!defined $self->{intr})
  {
    require Alien::Build::Interpolate::Default;
    $self->{intr} = Alien::Build::Interpolate::Default->new;
  }
  $self->{intr};
}


sub has_hook
{
  my($self, $name) = @_;
  defined $self->{hook}->{$name};
}


sub _instr
{
  my($self, $name, $instr) = @_;
  if(ref($instr) eq 'CODE')
  {
    return $instr;
  }
  elsif(ref($instr) eq 'ARRAY')
  {
    my %phase = (
      download      => 'share',
      fetch         => 'share',
      decode        => 'share',
      prefer        => 'share',
      extract       => 'share',
      patch         => 'share',
      patch_ffi     => 'share',
      build         => 'share',
      build_ffi     => 'share',
      stage         => 'share',
      gather_ffi    => 'share',
      gather_share  => 'share',
      gather_system => 'system',
      test_ffi      => 'share',
      test_share    => 'share',
      test_system   => 'system',
    );
    require Alien::Build::CommandSequence;
    my $seq = Alien::Build::CommandSequence->new(@$instr);
    $seq->apply_requirements($self, $phase{$name} || 'any');
    return $seq;
  }
  else
  {
    Carp::croak "type not supported as a hook";
  }
}

sub register_hook
{
  my($self, $name, $instr) = @_;
  push @{ $self->{hook}->{$name} }, _instr $self, $name, $instr;
  $self;
}


sub default_hook
{
  my($self, $name, $instr) = @_;
  $self->{default_hook}->{$name} = _instr $self, $name, $instr;
  $self;
}


sub around_hook
{
  my($self, $name, $code) = @_;
  if(my $old = $self->{around}->{$name})
  {
    # this is the craziest shit I have ever
    # come up with.
    $self->{around}->{$name} = sub {
      my $orig = shift;
      $code->(sub { $old->($orig, @_) }, @_);
    };
  }
  else
  {
    $self->{around}->{$name} = $code;
  }
}


sub after_hook
{
  my($self, $name, $code) = @_;
  $self->around_hook(
    $name => sub {
      my $orig = shift;
      my $ret = $orig->(@_);
      $code->(@_);
      $ret;
    }
  );
}


sub before_hook
{
  my($self, $name, $code) = @_;
  $self->around_hook(
    $name => sub {
      my $orig = shift;
      $code->(@_);
      my $ret = $orig->(@_);
      $ret;
    }
  );
}


sub call_hook
{
  my $self = shift;
  my %args = ref $_[0] ? %{ shift() } : ();
  my($name, @args) = @_;
  my $error;

  my @hooks = @{ $self->{hook}->{$name} || []};

  if(@hooks == 0)
  {
    if(defined $self->{default_hook}->{$name})
    {
      @hooks = ($self->{default_hook}->{$name})
    }
    elsif(!$args{all})
    {
      Carp::croak "No hooks registered for $name";
    }
  }

  my $value;

  foreach my $hook (@hooks)
  {
    if(eval { $args[0]->isa('Alien::Build') })
    {
      %{ $args[0]->{hook_prop} } = (
        name => $name,
      );
    }

    my $wrapper = $self->{around}->{$name} || sub { my $code = shift; $code->(@_) };
    my $value;
    $args{before}->() if $args{before};
    if(ref($hook) eq 'CODE')
    {
      $value = eval {
        my $value = $wrapper->(sub { $hook->(@_) }, @args);
        $args{verify}->('code') if $args{verify};
        $value;
      };
    }
    else
    {
      $value = $wrapper->(sub {
        eval {
          $hook->execute(@_);
          $args{verify}->('command') if $args{verify};
        };
        defined $args{ok} ? $args{ok} : 1;
      }, @args);
    }
    $error = $@;
    $args{after}->() if $args{after};
    if($args{all})
    {
      die if $error;
    }
    else
    {
      next if $error;
      next if $args{continue} && $args{continue}->($value);
      return $value;
    }
  }

  die $error if $error && ! $args{all};

  $value;
}


sub apply_plugin
{
  my($self, $name, @args) = @_;

  my $class;
  my $pm;
  my $found;

  if($name =~ /^=(.*)$/)
  {
    $class = $1;
    $pm    = "$class.pm";
    $pm    =~ s!::!/!g;
    $found = 1;
  }

  if($name !~ /::/ && !$found)
  {
    foreach my $inc (@INC)
    {
      # TODO: allow negotiators to work with @INC hooks
      next if ref $inc;
      my $file = Path::Tiny->new("$inc/Alien/Build/Plugin/$name/Negotiate.pm");
      if(-r $file)
      {
        $class = "Alien::Build::Plugin::${name}::Negotiate";
        $pm    = "Alien/Build/Plugin/$name/Negotiate.pm";
        $found = 1;
        last;
      }
    }
  }

  unless($found)
  {
    $class = "Alien::Build::Plugin::$name";
    $pm    = "Alien/Build/Plugin/$name.pm";
    $pm    =~ s{::}{/}g;
  }

  require $pm unless $class->can('new');
  my $plugin = $class->new(@args);
  $plugin->init($self);
  $self;
}

package Alien::Build::TempDir;

# TODO: it's confusing that there is both a AB::TempDir and AB::Temp
# although they do different things.  there could maybe be a better
# name for AB::TempDir (maybe AB::TempBuildDir, though that is a little
# redundant).  Happily both are private classes, and either are able to
# rename, if a good name can be thought of.

use overload '""' => sub { shift->as_string }, bool => sub { 1 }, fallback => 1;
use File::Temp qw( tempdir );

sub new
{
  my($class, $build, $name) = @_;
  my $root = $build->install_prop->{root};
  Path::Tiny->new($root)->mkpath unless -d $root;
  bless {
    dir => Path::Tiny->new(tempdir( "${name}_XXXX", DIR => $root)),
  }, $class;
}

sub as_string
{
  shift->{dir}->stringify;
}

sub DESTROY
{
  my($self) = @_;
  if(-d $self->{dir} && $self->{dir}->children == 0)
  {
    rmdir($self->{dir}) || warn "unable to remove @{[ $self->{dir} ]} $!";
  }
}

1;

__END__

=pod

=encoding UTF-8

=head1 NAME

Alien::Build - Build external dependencies for use in CPAN

=head1 VERSION

version 2.88

=head1 SYNOPSIS

 my $build = Alien::Build->load('./alienfile');
 $build->load_requires('configure');
 $build->set_prefix('/usr/local');
 $build->set_stage('/foo/mystage');  # needs to be absolute
 $build->load_requires($build->install_type);
 $build->download;
 $build->build;
 # files are now in /foo/mystage, it is your job (or
 # ExtUtils::MakeMaker, Module::Build, etc) to copy
 # those files into /usr/local

=head1 DESCRIPTION

This module provides tools for building external (non-CPAN) dependencies
for CPAN.  It is mainly designed to be used at install time of a CPAN
client, and work closely with L<Alien::Base> which is used at runtime.

This is the detailed documentation for the L<Alien::Build> class.
If you are starting out you probably want to do so from one of these documents:

=over 4

=item L<Alien::Build::Manual::Alien>

A broad overview of C<Alien-Build> and its ecosystem.

=item L<Alien::Build::Manual::AlienUser>

For users of an C<Alien::libfoo> that is implemented using L<Alien::Base>.
(The developer of C<Alien::libfoo> I<should> provide the documentation
necessary, but if not, this is the place to start).

=item L<Alien::Build::Manual::AlienAuthor>

If you are writing your own L<Alien> based on L<Alien::Build> and L<Alien::Base>.

=item L<Alien::Build::Manual::FAQ>

If you have a common question that has already been answered, like
"How do I use L<alienfile> with some build system".

=item L<Alien::Build::Manual::PluginAuthor>

This is for the brave souls who want to write plugins that will work with
L<Alien::Build> + L<alienfile>.

=item L<Alien::Build::Manual::Security>

If you are concerned that L<Alien>s might be downloading tarballs off
the internet, then this is the place for you.  This will discuss some
of the risks of downloading (really any) software off the internet
and will give you some tools to remediate these risks.

=back

Note that you will not usually create a L<Alien::Build> instance
directly, but rather be using a thin installer layer, such as
L<Alien::Build::MM> (for use with L<ExtUtils::MakeMaker>) or
L<Alien::Build::MB> (for use with L<Module::Build>).  One of the
goals of this project is to remain installer agnostic.

=head1 CONSTRUCTORS

=head2 new

 my $build = Alien::Build->new;

This creates a new empty instance of L<Alien::Build>.  Normally you will
want to use C<load> below to create an instance of L<Alien::Build> from
an L<alienfile> recipe.

=head2 load

 my $build = Alien::Build->load($alienfile);

This creates an L<Alien::Build> instance with the given L<alienfile>
recipe.

=head2 resume

 my $build = Alien::Build->resume($alienfile, $root);

Load a checkpointed L<Alien::Build> instance.  You will need the original
L<alienfile> and the build root (usually C<_alien>), and a build that
had been properly checkpointed using the C<checkpoint> method below.

=head1 PROPERTIES

There are three main properties for L<Alien::Build>.  There are a number
of properties documented here with a specific usage.  Note that these
properties may need to be serialized into something primitive like JSON
that does not support: regular expressions, code references of blessed
objects.

If you are writing a plugin (L<Alien::Build::Plugin>) you should use a
prefix like "plugin_I<name>" (where I<name> is the name of your plugin)
so that it does not interfere with other plugin or future versions of
L<Alien::Build>.  For example, if you were writing
C<Alien::Build::Plugin::Fetch::NewProtocol>, please use the prefix
C<plugin_fetch_newprotocol>:

 sub init
 {
   my($self, $meta) = @_;
 
   $meta->prop( plugin_fetch_newprotocol_foo => 'some value' );
 
   $meta->register_hook(
     some_hook => sub {
       my($build) = @_;
       $build->install_prop->{plugin_fetch_newprotocol_bar} = 'some other value';
       $build->runtime_prop->{plugin_fetch_newprotocol_baz} = 'and another value';
     }
   );
 }

If you are writing a L<alienfile> recipe please use the prefix C<my_>:

 use alienfile;
 
 meta_prop->{my_foo} = 'some value';
 
 probe sub {
   my($build) = @_;
   $build->install_prop->{my_bar} = 'some other value';
   $build->install_prop->{my_baz} = 'and another value';
 };

Any property may be used from a command:

 probe [ 'some command %{.meta.plugin_fetch_newprotocol_foo}' ];
 probe [ 'some command %{.install.plugin_fetch_newprotocol_bar}' ];
 probe [ 'some command %{.runtime.plugin_fetch_newprotocol_baz}' ];
 probe [ 'some command %{.meta.my_foo}' ];
 probe [ 'some command %{.install.my_bar}' ];
 probe [ 'some command %{.runtime.my_baz}' ];

=head2 meta_prop

 my $href = $build->meta_prop;
 my $href = Alien::Build->meta_prop;

Meta properties have to do with the recipe itself, and not any particular
instance that probes or builds that recipe.  Meta properties can be changed
from within an L<alienfile> using the C<meta_prop> directive, or from
a plugin from its C<init> method (though should NOT be modified from any
hooks registered within that C<init> method).  This is not strictly enforced,
but if you do not follow this rule your recipe will likely be broken.

=over

=item arch

This is a hint to an installer like L<Alien::Build::MM> or L<Alien::Build::MB>,
that the library or tool contains architecture dependent files and so should
be stored in an architecture dependent location.  If not specified by your
L<alienfile> then it will be set to true.

=item check_digest

True if cryptographic digest should be checked when files are fetched
or downloaded.  This is set by
L<Digest negotiator plugin|Alien::Build::Plugin::Digest::Negotiate>.

=item destdir

Some plugins (L<Alien::Build::Plugin::Build::Autoconf> for example) support
installing via C<DESTDIR>.  They will set this property to true if they
plan on doing such an install.  This helps L<Alien::Build> find the staged
install files and how to locate them.

If available, C<DESTDIR> is used to stage install files in a sub directory before
copying the files into C<blib>.  This is generally preferred method
if available.

=item destdir_filter

Regular expression for the files that should be copied from the C<DESTDIR>
into the stage directory.  If not defined, then all files will be copied.

=item destdir_ffi_filter

Same as C<destdir_filter> except applies to C<build_ffi> instead of C<build>.

=item digest

This properties contains the cryptographic digests (if any) that should
be used when verifying any fetched and downloaded files.  It is a hash
reference where the key is the filename and the value is an array
reference containing a pair of values, the first being the algorithm
('SHA256' is recommended) and the second is the actual digest.  The
special filename C<*> may be specified to indicate that any downloaded
file should match that digest.  If there are both real filenames and
the C<*> placeholder, the real filenames will be used for filenames
that match and any other files will use the placeholder.  Example:

 $build->meta_prop->{digest} = {
   'foo-1.00.tar.gz' => [ SHA256 => '9feac593aa49a44eb837de52513a57736457f1ea70078346c60f0bfc5f24f2c1' ],
   'foo-1.01.tar.gz' => [ SHA256 => '6bbde6a7f10ae5924cf74afc26ff5b7bc4b4f9dfd85c6b534c51bd254697b9e7' ],
   '*'               => [ SHA256 => '33a20aae3df6ecfbe812b48082926d55391be4a57d858d35753ab1334b9fddb3' ],
 };

Cryptographic signatures will only be checked
if the L<check_digest meta property|/check_digest> is set and if the
L<Digest negotiator plugin|Alien::Build::Plugin::Digest::Negotiate> is loaded.
(The Digest negotiator can be used directly, but is also loaded automatically
if you use the L<digest directive|alienfile/digest> is used by the L<alienfile>).

=item env

Environment variables to override during the build stage.

=item env_interpolate

Environment variable values will be interpolated with helpers.  Example:

 meta->prop->{env_interpolate} = 1;
 meta->prop->{env}->{PERL} = '%{perl}';

=item local_source

Set to true if source code package is available locally.  (that is not fetched
over the internet).  This is computed by default based on the C<start_url>
property.  Can be set by an L<alienfile> or plugin.

=item platform

Hash reference.  Contains information about the platform beyond just C<$^O>.

=over 4

=item platform.compiler_type

Refers to the type of flags that the compiler accepts.  May be expanded in the
future, but for now, will be one of:

=over 4

=item microsoft

On Windows when using Microsoft Visual C++

=item unix

Virtually everything else, including gcc on windows.

=back

The main difference is that with Visual C++ C<-LIBPATH> should be used instead
of C<-L>, and static libraries should have the C<.LIB> suffix instead of C<.a>.

=item platform.system_type

C<$^O> is frequently good enough to make platform specific logic in your
L<alienfile>, this handles the case when $^O can cover platforms that provide
multiple environments that Perl might run under.  The main example is windows,
but others may be added in the future.

=over 4

=item unix

=item vms

=item windows-activestate

=item windows-microsoft

=item windows-mingw

=item windows-strawberry

=item windows-unknown

=back

Note that C<cygwin> and C<msys> are considered C<unix> even though they run
on windows!

=item platform.cpu.count

Contains a non-negative integer of available (possibly virtual) CPUs on the
system. This can be used by build plugins to build in parallel. The environment
variable C<ALIEN_CPU_COUNT> can be set to override the CPU count.

=item platform.cpu.arch.name

Contains a normalized name for the architecture of the current Perl. This can
be used by fetch plugins to determine which binary packages to download.
The value may be one of the following, but this list will be expanded as
needed.

=over 4

=item C<armel>

32-bit ARM soft-float

=item C<armhf>

32-bit ARM hard-float

=item C<aarch64>

64-bit ARM

=item C<ppc>

32-bit PowerPC (big-endian)

=item C<ppc64>

64-bit PowerPC (big-endian)

=item C<x86>

32-bit Intel (i386, i486, i686)

=item C<x86_64>

64-bit Intel (AMD64)

=item C<unknown>

Unable to detect architecture. Please report this if needed.

=back

=back

=item out_of_source

Build in a different directory from the where the source code is stored.
In autoconf this is referred to as a "VPATH" build.  Everyone else calls this
an "out-of-source" build.  When this property is true, instead of extracting
to the source build root, the downloaded source will be extracted to an source
extraction directory and the source build root will be empty.  You can use the
C<extract> install property to get the location of the extracted source.

=item network

True if a network fetch is available.  This should NOT be set by an L<alienfile>
or plugin.  This is computed based on the C<ALIEN_INSTALL_NETWORK> environment
variables.

=item start_url

The default or start URL used by fetch plugins.

=back

=head2 install_prop

 my $href = $build->install_prop;

Install properties are used during the install phase (either
under C<share> or C<system> install).  They are remembered for
the entire install phase, but not kept around during the runtime
phase.  Thus they cannot be accessed from your L<Alien::Base>
based module.

=over

=item autoconf_prefix

The prefix as understood by autoconf.  This is only different on Windows
Where MSYS is used and paths like C<C:/foo> are  represented as C</C/foo>
which are understood by the MSYS tools, but not by Perl.  You should
only use this if you are using L<Alien::Build::Plugin::Build::Autoconf> in
your L<alienfile>.  This is set during before the
L<build hook|Alien::Build::Manual::PluginAuthor/"build hook"> is run.

=item download

The location of the downloaded archive (tar.gz, or similar) or directory.
This will be undefined until the archive is actually downloaded.

=item download_detail

This property contains optional details about a downloaded file.  This
property is populated by L<Alien::Build> core.  This property is a
hash reference.  The key is the path to the file that has been downloaded
and the value is a hash reference with additional detail.  All fields
are optional.

=over 4

=item download_detail.digest

This, if available, with the cryptographic signature that was successfully
matched against the downloaded file.  It is an array reference with a
pair of values, the algorithm (typically something like C<SHA256>) and
the digest.

=item download_detail.protocol

This, if available, will be the URL protocol used to fetch the downloaded
file.

=back

=item env

Environment variables to override during the build stage.  Plugins are
free to set additional overrides using this hash.

=item extract

The location of the last source extraction.  For a "out-of-source" build
(see the C<out_of_source> meta property above), this will only be set once.
For other types of builds, the source code may be extracted multiple times,
and thus this property may change.

=item old

[deprecated]

Hash containing information on a previously installed Alien of the same
name, if available.  This may be useful in cases where you want to
reuse the previous install if it is still sufficient.

=over 4

=item old.prefix

[deprecated]

The prefix for the previous install.  Versions prior to 1.42 unfortunately
had this in typo form of C<preifx>.

=item old.runtime

[deprecated]

The runtime properties from the previous install.

=back

=item patch

Directory with patches, if available.  This will be C<undef> if there
are no patches.  When initially installing an alien this will usually
be a sibling of the C<alienfile>, a directory called C<patch>.  Once
installed this will be in the share directory called C<_alien/patch>.
The former is useful for rebuilding an alienized package using
L<af>.

=item prefix

The install time prefix.  Under a C<destdir> install this is the
same as the runtime or final install location.  Under a non-C<destdir>
install this is the C<stage> directory (usually the appropriate
share directory under C<blib>).

=item root

The build root directory.  This will be an absolute path.  It is the
absolute form of C<./_alien> by default.

=item stage

The stage directory where files will be copied.  This is usually the
root of the blib share directory.

=item system_probe_class

After the probe step this property may contain the plugin class that
performed the system probe.  It shouldn't be filled in directly by
the plugin (instead if should use the hook property C<probe_class>,
see below).  This is optional, and not all probe plugins will provide
this information.

=item system_probe_instance_id

After the probe step this property may contain the plugin instance id that
performed the system probe.  It shouldn't be filled in directly by
the plugin (instead if should use the hook property C<probe_instance_id>,
see below).  This is optional, and not all probe plugins will provide
this information.

=back

=head2 plugin_instance_prop

 my $href = $build->plugin_instance_prop($plugin);

This returns the private plugin instance properties for a given plugin.
This method should usually only be called internally by plugins themselves
to keep track of internal state.  Because the content can be used arbitrarily
by the owning plugin because it is private to the plugin, and thus is not
part of the L<Alien::Build> spec.

=head2 runtime_prop

 my $href = $build->runtime_prop;

Runtime properties are used during the install and runtime phases
(either under C<share> or C<system> install).  This should include
anything that you will need to know to use the library or tool
during runtime, and shouldn't include anything that is no longer
relevant once the install process is complete.

=over 4

=item alien_build_version

The version of L<Alien::Build> used to install the library or tool.

=item alt

Alternate configurations.  If the alienized package has multiple
libraries this could be used to store the different compiler or
linker flags for each library.  Typically this will be set by a
plugin in the gather stage (for either share or system installs).

=item cflags

The compiler flags.  This is typically set by a plugin in the
gather stage (for either share or system installs).

=item cflags_static

The static compiler flags.  This is typically set by a plugin in the
gather stage (for either share or system installs).

=item command

The command name for tools where the name my differ from platform to
platform.  For example, the GNU version of make is usually C<make> in
Linux and C<gmake> on FreeBSD.  This is typically set by a plugin in the
gather stage (for either share or system installs).

=item ffi_name

The name DLL or shared object "name" to use when searching for dynamic
libraries at runtime.  This is passed into L<FFI::CheckLib>, so if
your library is something like C<libarchive.so> or C<archive.dll> you
would set this to C<archive>.  This may be a string or an array of
strings.  This is typically set by a plugin in the gather stage
(for either share or system installs).

=item ffi_checklib

This property contains two sub properties:

=over 4

=item ffi_checklib.share

 $build->runtime_prop->{ffi_checklib}->{share} = [ ... ];

Array of additional L<FFI::CheckLib> flags to pass in to C<find_lib>
for a C<share> install.

=item ffi_checklib.system

Array of additional L<FFI::CheckLib> flags to pass in to C<find_lib>
for a C<system> install.

Among other things, useful for specifying the C<try_linker_script>
flag:

 $build->runtime_prop->{ffi_checklib}->{system} = [ try_linker_script => 1 ];

=back

This is typically set by a plugin in the gather stage
(for either share or system installs).

=item inline_auto_include

[version 2.53]

This property is an array reference of C code that will be passed into
L<Inline::C> to make sure that appropriate headers are automatically
included.  See L<Inline::C/auto_include> for details.

=item install_type

The install type.  This is set by AB core after the
L<probe hook|Alien::Build::Manual::PluginAuthor/"probe hook"> is
executed.  Is one of:

=over 4

=item system

For when the library or tool is provided by the operating system, can be
detected by L<Alien::Build>, and is considered satisfactory by the
C<alienfile> recipe.

=item share

For when a system install is not possible, the library source will be
downloaded from the internet or retrieved in another appropriate fashion
and built.

=back

=item libs

The library flags.  This is typically set by a plugin in the
gather stage (for either share or system installs).

=item libs_static

The static library flags.  This is typically set by a plugin in the
gather stage (for either share or system installs).

=item perl_module_version

The version of the Perl module used to install the alien (if available).
For example if L<Alien::curl> is installing C<libcurl> this would be the
version of L<Alien::curl> used during the install step.

=item prefix

The final install root.  This is usually they share directory.

=item version

The version of the library or tool.  This is typically set by a plugin in the
gather stage (for either share or system installs).

=back

=head2 hook_prop

 my $href = $build->hook_prop;

Hook properties are for the currently running (if any) hook.  They are
used only during the execution of each hook and are discarded after.
If no hook is currently running then C<hook_prop> will return C<undef>.

=over 4

=item name

The name of the currently running hook.

=item version (probe)

Probe and PkgConfig plugins I<may> set this property indicating the
version of the alienized package.  Not all plugins and configurations
may be able to provide this.

=item probe_class (probe)

Probe and PkgConfig plugins I<may> set this property indicating the
plugin class that made the probe.  If the probe results in a system
install this will be propagated to C<system_probe_class> for later
use.

=item probe_instance_id (probe)

Probe and PkgConfig plugins I<may> set this property indicating the
plugin instance id that made the probe.  If the probe results in a
system install this will be propagated to C<system_probe_instance_id>
for later use.

=back

=head1 METHODS

=head2 checkpoint

 $build->checkpoint;

Save any install or runtime properties so that they can be reloaded on
a subsequent run in a separate process.  This is useful if your build
needs to be done in multiple stages from a C<Makefile>, such as with
L<ExtUtils::MakeMaker>.  Once checkpointed you can use the C<resume>
constructor (documented above) to resume the probe/build/install]
process.

=head2 root

 my $dir = $build->root;

This is just a shortcut for:

 my $root = $build->install_prop->{root};

Except that it will be created if it does not already exist.

=head2 install_type

 my $type = $build->install_type;

This will return the install type.  (See the like named install property
above for details).  This method will call C<probe> if it has not already
been called.

=head2 is_system_install

 my $boolean = $build->is_system_install;

Returns true if the alien is a system install type.  

=head2 is_share_install

 my $boolean = $build->is_share_install;

Returns true if the alien is a share install type.

=head2 download_rule

 my $rule = $build->download_rule;

This returns install rule as a string.  This is determined by the environment
and should be one of:

=over 4

=item C<warn>

Warn only if fetching via non secure source (secure sources include C<https>,
and bundled files, may include other encrypted protocols in the future).

=item C<digest>

Require that any downloaded source package have a cryptographic signature in
the L<alienfile> and that signature matches what was downloaded.

=item C<encrypt>

Require that any downloaded source package is fetched via secure source.

=item C<digest_or_encrypt>

Require that any downloaded source package is B<either> fetched via a secure source
B<or> has a cryptographic signature in the L<alienfile> and that signature matches
what was downloaded.

=item C<digest_and_encrypt>

Require that any downloaded source package is B<both> fetched via a secure source
B<and> has a cryptographic signature in the L<alienfile> and that signature matches
what was downloaded.

=back

The current default is C<warn>, but in the near future this will be upgraded to
C<digest_or_encrypt>.

=head2 set_prefix

 $build->set_prefix($prefix);

Set the final (unstaged) prefix.  This is normally only called by L<Alien::Build::MM>
and similar modules.  It is not intended for use from plugins or from an L<alienfile>.

=head2 set_stage

 $build->set_stage($dir);

Sets the stage directory.  This is normally only called by L<Alien::Build::MM>
and similar modules.  It is not intended for use from plugins or from an L<alienfile>.

=head2 requires

 my $hash = $build->requires($phase);

Returns a hash reference of the modules required for the given phase.  Phases
include:

=over 4

=item configure

These modules must already be available when the L<alienfile> is read.

=item any

These modules are used during either a C<system> or C<share> install.

=item share

These modules are used during the build phase of a C<share> install.

=item system

These modules are used during the build phase of a C<system> install.

=back

=head2 load_requires

 $build->load_requires($phase);

This loads the appropriate modules for the given phase (see C<requires> above
for a description of the phases).

=head2 probe

 my $install_type = $build->probe;

Attempts to determine if the operating system has the library or
tool already installed.  If so, then the string C<system> will
be returned and a system install will be performed.  If not,
then the string C<share> will be installed and the tool or
library will be downloaded and built from source.

If the environment variable C<ALIEN_INSTALL_TYPE> is set, then that
will force a specific type of install.  If the detection logic
cannot accommodate the install type requested then it will fail with
an exception.

=head2 download

 $build->download;

Download the source, usually as a tarball, usually from the internet.

Under a C<system> install this does not do anything.

=head2 fetch

 my $res = $build->fetch;
 my $res = $build->fetch($url, %options);

Fetch a resource using the fetch hook.  Returns the same hash structure
described below in the
L<fetch hook|Alien::Build::Manual::PluginAuthor/"fetch hook"> documentation.

[version 2.39]

As of L<Alien::Build> 2.39, these options are supported:

=over 4

=item http_headers

 my $res = $build->fetch($url, http_headers => [ $key1 => $value1, $key2 => $value 2, ... ]);

Set the HTTP request headers on all outgoing HTTP requests.  Note that not all
protocols or fetch plugins support setting request headers, but the ones that
do not I<should> issue a warning if you try to set request headers and they
are not supported.

=back

=head2 check_digest

[experimental]

 my $bool = $build->check_digest($path);

Checks any cryptographic signatures for the given file.  The
file is specified by C<$path> which may be one of:

=over 4

=item string

Containing the path to the file to be checked.

=item L<Path::Tiny>

Containing the path to the file to be checked.

=item C<HASH>

A Hash reference containing information about a file.  See
the L<fetch hook|Alien::Build::Manual::PluginAuthor/"fetch hook"> for details
on the format.

=back

Returns true if the cryptographic signature matches, false if cryptographic
signatures are disabled.  Will throw an exception if the signature does not
match, or if no plugin provides the correct algorithm for checking the
signature.

=head2 decode

 my $decoded_res = $build->decode($res);

Decode the HTML or file listing returned by C<fetch>.  Returns the same
hash structure described below in the
L<decode hook|Alien::Build::Manual::PluginAuthor/"decode hook"> documentation.

=head2 prefer

 my $sorted_res = $build->prefer($res);

Filter and sort candidates.  The preferred candidate will be returned first in the list.
The worst candidate will be returned last.  Returns the same hash structure described
below in the
L<prefer hook|Alien::Build::Manual::PluginAuthor/"prefer hook"> documentation.

=head2 extract

 my $dir = $build->extract;
 my $dir = $build->extract($archive);

Extracts the given archive into a fresh directory.  This is normally called internally
to L<Alien::Build>, and for normal usage is not needed from a plugin or L<alienfile>.

=head2 build

 $build->build;

Run the build step.  It is expected that C<probe> and C<download>
have already been performed.  What it actually does depends on the
type of install:

=over 4

=item share

The source is extracted, and built as determined by the L<alienfile>
recipe.  If there is a C<gather_share> that will be executed last.

=item system

The
L<gather_system hook|Alien::Build::Manual::PluginAuthor/"gather_system hook">
will be executed.

=back

=head2 test

 $build->test;

Run the test phase

=head2 clean_install

 $build->clean_install

Clean files from the final install location.  The default implementation removes all
files recursively except for the C<_alien> directory.  This is helpful when you have
an old install with files that may break the new build.

=head2 system

 $build->system($command);
 $build->system($command, @args);

Interpolates the command and arguments and run the results using
the Perl C<system> command.

=head2 log

 $build->log($message);

Send a message to the log.  By default this prints to C<STDOUT>.

=head2 meta

 my $meta = Alien::Build->meta;
 my $meta = $build->meta;

Returns the meta object for your L<Alien::Build> class or instance.  The
meta object is a way to manipulate the recipe, and so any changes to the
meta object should be made before the C<probe>, C<download> or C<build> steps.

=head1 META METHODS

=head2 prop

 my $href = $build->meta->prop;

Meta properties.  This is the same as calling C<meta_prop> on
the class or L<Alien::Build> instance.

=head2 add_requires

 Alien::Build->meta->add_requires($phase, $module => $version, ...);

Add the requirement to the given phase.  Phase should be one of:

=over 4

=item configure

=item any

=item share

=item system

=back

=head2 has_requires

 if (Alien::Build->meta->has_requires($phase, $module_pat, ...)) {...}

Tests to see if the given phase has any of the given substrings as
a requirement. Phase as L</add_requires>.

=head2 interpolator

 my $interpolator = $build->meta->interpolator;
 my $interpolator = Alien::Build->interpolator;

Returns the L<Alien::Build::Interpolate> instance for the L<Alien::Build> class.

=head2 has_hook

 my $bool = $build->meta->has_hook($name);
 my $bool = Alien::Build->has_hook($name);

Returns if there is a usable hook registered with the given name.

=head2 register_hook

 $build->meta->register_hook($name, $instructions);
 Alien::Build->meta->register_hook($name, $instructions);

Register a hook with the given name.  C<$instruction> should be either
a code reference, or a command sequence, which is an array reference.

=head2 default_hook

 $build->meta->default_hook($name, $instructions);
 Alien::Build->meta->default_hook($name, $instructions);

Register a default hook, which will be used if the L<alienfile> does not
register its own hook with that name.

=head2 around_hook

 $build->meta->around_hook($hook_name, $code);
 Alien::Build->meta->around_hook($hook_name, $code);

Wrap the given hook with a code reference.  This is similar to a L<Moose>
method modifier, except that it wraps around the given hook instead of
a method.  For example, this will add a probe system requirement:

 $build->meta->around_hook(
   probe => sub {
     my $orig = shift;
     my $build = shift;
     my $type = $orig->($build, @_);
     return $type unless $type eq 'system';
     # also require a configuration file
     if(-f '/etc/foo.conf')
     {
       return 'system';
     }
     else
     {
       return 'share';
     }
   },
 );

=head2 after_hook

 $build->meta->after_hook($hook_name, sub {
   my(@args) = @_;
   ...
 });

Execute the given code reference after the hook.  The original
arguments are passed into the code reference.

=head2 before_hook

 $build->meta->before_hook($hook_name, sub {
   my(@args) = @_;
   ...
 });

Execute the given code reference before the hook.  The original
arguments are passed into the code reference.

=head2 apply_plugin

 Alien::Build->meta->apply_plugin($name);
 Alien::Build->meta->apply_plugin($name, @args);

Apply the given plugin with the given arguments.

=head1 ENVIRONMENT

L<Alien::Build> responds to these environment variables:

=over 4

=item ALIEN_BUILD_LOG

The default log class used.  See L<Alien::Build::Log> and L<Alien::Build::Log::Default>.

=item ALIEN_BUILD_PKG_CONFIG

Override the logic in L<Alien::Build::Plugin::PkgConfig::Negotiate> which
chooses the best C<pkg-config> plugin.

=item ALIEN_BUILD_POSTLOAD

semicolon separated list of plugins to automatically load after parsing
your L<alienfile>.

=item ALIEN_BUILD_PRELOAD

semicolon separated list of plugins to automatically load before parsing
your L<alienfile>.

=item ALIEN_BUILD_RC

Perl source file which can override some global defaults for L<Alien::Build>,
by, for example, setting preload and postload plugins.

=item ALIEN_DOWNLOAD_RULE

This value determines the rules by which types of downloads are allowed.  The legal
values listed under L</download_rule>, plus C<default> which will be the default for
the current version of L<Alien::Build>.  For this version that default is C<warn>.

=item ALIEN_INSTALL_NETWORK

If set to true (the default), then network fetch will be allowed.  If set to
false, then network fetch will not be allowed.

What constitutes a local vs. network fetch is determined based on the C<start_url>
and C<local_source> meta properties.  An L<alienfile> or plugin C<could> override
this detection (possibly inappropriately), so this variable is not a substitute
for properly auditing of Perl modules for environments that require that.

=item ALIEN_INSTALL_TYPE

If set to C<share> or C<system>, it will override the system detection logic.
If set to C<default>, it will use the default setting for the L<alienfile>.
The behavior of other values is undefined.

Although the recommended way for a consumer to use an L<Alien::Base> based L<Alien>
is to declare it as a static configure and build-time dependency, some consumers
may prefer to fallback on using an L<Alien> only when the consumer itself cannot
detect the necessary package. In some cases the consumer may want the user to opt-in
to using an L<Alien> before requiring it.

To keep the interface consistent among Aliens, the consumer of the fallback opt-in
L<Alien> may fallback on the L<Alien> if the environment variable C<ALIEN_INSTALL_TYPE>
is set to any value. The rationale is that by setting this environment variable the
user is aware that L<Alien> modules may be installed and have indicated consent.
The actual implementation of this, by its nature would have to be in the consuming
CPAN module.

=item DESTDIR

This environment variable will be manipulated during a destdir install.

=item PKG_CONFIG

This environment variable can be used to override the program name for C<pkg-config>
when using the command line plugin: L<Alien::Build::Plugin::PkgConfig::CommandLine>.

=item ftp_proxy, all_proxy

If these environment variables are set, it may influence the Download negotiation
plugin L<Alien::Build::Plugin::Download::Negotiate>.  Other proxy variables may
be used by some Fetch plugins, if they support it.

=back

=head1 SUPPORT

The intent of the C<Alien-Build> team is to support the same versions of
Perl that are supported by the Perl toolchain.  As of this writing that
means 5.16 and better.

Please feel encouraged to report issues that you encounter to the
project GitHub Issue tracker:

=over 4

=item L<https://github.com/PerlAlien/Alien-Build/issues>

=back

Better if you can fix the issue yourself, please feel encouraged to open
pull-request on the project GitHub:

=over 4

=item L<https://github.com/PerlAlien/Alien-Build/pulls>

=back

If you are confounded and have questions, join us on the C<#native>
channel on irc.perl.org.  The C<Alien-Build> developers frequent this
channel and can probably help point you in the right direction.  If you
don't have an IRC client handy, you can use this web interface:

=over 4

=item L<https://chat.mibbit.com/?channel=%23native&server=irc.perl.org>

=back

=head1 SEE ALSO

L<Alien::Build::Manual::AlienAuthor>,
L<Alien::Build::Manual::AlienUser>,
L<Alien::Build::Manual::Contributing>,
L<Alien::Build::Manual::FAQ>,
L<Alien::Build::Manual::PluginAuthor>

L<alienfile>, L<Alien::Build::MM>, L<Alien::Build::Plugin>, L<Alien::Base>, L<Alien>

=head1 THANKS

L<Alien::Base> was originally written by Joel Berger, the rest of this project would
not have been possible without him getting the project started.  Thanks to his support
I have been able to augment the original L<Alien::Base> system with a reliable set
of tools (L<Alien::Build>, L<alienfile>, L<Test::Alien>), which make up this toolset.

The original L<Alien::Base> is still copyright (c) 2012-2020 Joel Berger.  It has
the same license as the rest of the Alien::Build and related tools distributed as
C<Alien-Build>.  Joel Berger thanked a number of people who helped in in the development
of L<Alien::Base>, in the documentation for that module.

I would also like to acknowledge the other members of the PerlAlien github
organization, Zakariyya Mughal (sivoais, ZMUGHAL) and mohawk (ETJ).  Also important
in the early development of L<Alien::Build> were the early adopters Chase Whitener
(genio, CAPOEIRAB, author of L<Alien::libuv>), William N. Braswell, Jr (willthechill,
WBRASWELL, author of L<Alien::JPCRE2> and L<Alien::PCRE2>) and Ahmad Fatoum (a3f,
ATHREEF, author of L<Alien::libudev> and L<Alien::LibUSB>).

The Alien ecosystem owes a debt to Dan Book, who goes by Grinnz on IRC, for answering
question about how to use L<Alien::Build> and friends.

=head1 AUTHOR

Author: Graham Ollis E<lt>plicease@cpan.orgE<gt>

Contributors:

Diab Jerius (DJERIUS)

Roy Storey (KIWIROY)

Ilya Pavlov

David Mertens (run4flat)

Mark Nunberg (mordy, mnunberg)

Christian Walde (Mithaldu)

Brian Wightman (MidLifeXis)

Zaki Mughal (zmughal)

mohawk (mohawk2, ETJ)

Vikas N Kumar (vikasnkumar)

Flavio Poletti (polettix)

Salvador Fandiño (salva)

Gianni Ceccarelli (dakkar)

Pavel Shaydo (zwon, trinitum)

Kang-min Liu (劉康民, gugod)

Nicholas Shipp (nshp)

Juan Julián Merelo Guervós (JJ)

Joel Berger (JBERGER)

Petr Písař (ppisar)

Lance Wicks (LANCEW)

Ahmad Fatoum (a3f, ATHREEF)

José Joaquín Atria (JJATRIA)

Duke Leto (LETO)

Shoichi Kaji (SKAJI)

Shawn Laffan (SLAFFAN)

Paul Evans (leonerd, PEVANS)

Håkon Hægland (hakonhagland, HAKONH)

nick nauwelaerts (INPHOBIA)

Florian Weimer

Marcel Telka (mtelka)

=head1 COPYRIGHT AND LICENSE

This software is copyright (c) 2011-2022 by Graham Ollis.

This is free software; you can redistribute it and/or modify it under
the same terms as the Perl 5 programming language system itself.

=cut
