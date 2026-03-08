package cfgmgr_test

import (
	"os"
	"path"
	"testing"
	"time"

	"github.com/fsnotify/fsnotify"
	"github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/shared/events"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
)

type TestCfg struct {
	Field string `json:"field"`

	postLoad func() error
	preSave  func() error
}

func (tc *TestCfg) Default() cfgmgr.Configer {
	return &TestCfg{}
}
func (tc *TestCfg) PostLoad() error {
	if tc.postLoad != nil {
		return tc.postLoad()
	}
	return nil
}
func (tc *TestCfg) PreSave() error {
	if tc.preSave != nil {
		return tc.preSave()
	}
	return nil
}
func (tc *TestCfg) PostUpdate(cfgmgr.Configer, fsnotify.Event) cfgmgr.ConfigChangedType {
	return cfgmgr.ConfigChangedGeneric
}

func TempFile(t *testing.T, content string) *os.File {
	f, err := os.CreateTemp(t.TempDir(), "")
	assert.NoError(t, err)
	t.Cleanup(func() {
		assert.NoError(t, f.Close())
		assert.NoError(t, os.Remove(f.Name()))
	})
	if content != "" {
		n, err := f.WriteString(content)
		assert.Equal(t, len(content), n)
		assert.NoError(t, err)
	}
	return f
}

func TestConfig_Load(t *testing.T) {
	f := TempFile(t, `{"field":"foo"}`)

	mgr, err := cfgmgr.NewManager[*TestCfg](f.Name())
	assert.NoError(t, err)
	assert.Equal(t, &TestCfg{
		Field: "foo",
	}, mgr.Get())

	mgr.Get().Field = "fo"
	assert.NoError(t, mgr.Save())
}

func TestConfig_Hooks(t *testing.T) {
	f := TempFile(t, `{"field":"foo"}`)

	mgr, err := cfgmgr.NewManager[*TestCfg](f.Name())
	assert.NoError(t, err)
	assert.Equal(t, &TestCfg{
		Field: "foo",
	}, mgr.Get())

	postLoad := false
	preSave := false
	mgr.Get().postLoad = func() error {
		postLoad = true
		return nil
	}
	mgr.Get().preSave = func() error {
		preSave = true
		return nil
	}
	assert.NoError(t, mgr.Save())
	assert.True(t, preSave)

	assert.NoError(t, mgr.Load())
	assert.True(t, postLoad)
}

func TestConfig_Load_Invalid(t *testing.T) {
	f := TempFile(t, `{"field":"foo}`)

	mgr, err := cfgmgr.NewManager[*TestCfg](f.Name())
	assert.Error(t, err)
	assert.Nil(t, mgr)
}

func TestConfig_Reload(t *testing.T) {
	logrus.SetLevel(logrus.DebugLevel)
	f := TempFile(t, `{"field":"foo"}`)

	mgr, err := cfgmgr.NewManager[*TestCfg](f.Name())
	assert.NoError(t, err)
	assert.Equal(t, &TestCfg{Field: "foo"}, mgr.Get())

	called := false
	mgr.Bus().AddEventListener(cfgmgr.TopicConfigChanged, func(ev *events.Event) {
		called = true
	})

	_, err = os.CreateTemp(path.Dir(f.Name()), "")
	assert.NoError(t, err)
	time.Sleep(5 * time.Second)
	assert.True(t, called)
}
